use crate::db::SyncMode;
use crate::storage::FileInfo;
use std::collections::{HashMap, HashSet};

/// 同步动作
#[derive(Debug, Clone)]
pub enum SyncAction {
    /// 复制文件：源路径 -> 目标路径
    Copy {
        source_path: String,
        dest_path: String,
        size: u64,
        /// 是否是从目标复制到源（双向同步时）
        reverse: bool,
    },
    /// 删除文件
    Delete {
        path: String,
        /// 删除目标还是源
        from_dest: bool,
    },
    /// 跳过（文件相同）
    Skip { path: String },
    /// 冲突（需要用户决定）
    Conflict {
        path: String,
        source_info: Option<FileInfo>,
        dest_info: Option<FileInfo>,
        conflict_type: ConflictType,
    },
}

/// 冲突类型
#[derive(Debug, Clone)]
pub enum ConflictType {
    /// 两边都修改了
    BothModified,
    /// 大小相同但时间不同（可能是同一内容）
    SameSizeDifferentTime,
    /// 一边修改一边删除
    ModifiedVsDeleted,
}

/// 文件比较结果
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileRelation {
    /// 文件相同
    Equal,
    /// 源文件更新
    SourceNewer,
    /// 目标文件更新
    DestNewer,
    /// 不同（大小不同）
    Different,
    /// 可能相同（大小相同，时间差异小）
    ProbablyEqual,
}

/// 比较配置
#[derive(Debug, Clone)]
pub struct CompareConfig {
    /// 时间容差（秒）
    pub time_tolerance_seconds: i64,
    /// 是否使用 checksum 比较
    pub use_checksum: bool,
    /// 是否忽略文件时间（仅比较大小和 checksum）
    pub ignore_mtime: bool,
    /// 大小相同时是否认为文件相同（适用于 WebDAV 等不保留 mtime 的场景）
    pub size_only_for_same_size: bool,
}

impl Default for CompareConfig {
    fn default() -> Self {
        Self {
            time_tolerance_seconds: 2,
            use_checksum: false,
            ignore_mtime: false,
            size_only_for_same_size: true, // 默认开启，避免 WebDAV 重复同步
        }
    }
}

/// 文件比较器
pub struct FileComparator {
    config: CompareConfig,
}

impl FileComparator {
    pub fn new(use_checksum: bool) -> Self {
        Self {
            config: CompareConfig {
                use_checksum,
                ..Default::default()
            },
        }
    }

    pub fn with_config(config: CompareConfig) -> Self {
        Self { config }
    }

    /// 比较两个文件
    pub fn compare_files(&self, source: &FileInfo, dest: &FileInfo) -> FileRelation {
        // 首先检查 checksum（如果有）
        if self.config.use_checksum {
            if let (Some(src_sum), Some(dst_sum)) = (&source.checksum, &dest.checksum) {
                if src_sum == dst_sum {
                    return FileRelation::Equal;
                } else {
                    return FileRelation::Different;
                }
            }
        }

        // 大小不同，肯定不同
        if source.size != dest.size {
            tracing::debug!(
                "文件大小不同: {} (src={}, dst={})",
                source.path,
                source.size,
                dest.size
            );
            return FileRelation::Different;
        }

        // 大小相同时，如果开启了 size_only_for_same_size，直接认为相同
        // 这适用于 WebDAV 等不保留原始修改时间的存储
        if self.config.size_only_for_same_size {
            return FileRelation::Equal;
        }

        // 如果忽略时间，大小相同就认为相同
        if self.config.ignore_mtime {
            return FileRelation::ProbablyEqual;
        }

        // 比较修改时间
        let time_diff = (source.modified_time - dest.modified_time).abs();

        if time_diff <= self.config.time_tolerance_seconds {
            return FileRelation::Equal;
        }

        tracing::debug!(
            "文件时间不同: {} (src_time={}, dst_time={}, diff={}s)",
            source.path,
            source.modified_time,
            dest.modified_time,
            time_diff
        );

        if source.modified_time > dest.modified_time {
            FileRelation::SourceNewer
        } else {
            FileRelation::DestNewer
        }
    }

    /// 比较两个文件树，返回同步动作列表
    pub fn compare_trees(
        &self,
        source: &HashMap<String, FileInfo>,
        dest: &HashMap<String, FileInfo>,
        mode: &SyncMode,
    ) -> Vec<SyncAction> {
        let mut actions = Vec::new();

        // 收集所有路径
        let all_paths: HashSet<_> = source.keys().chain(dest.keys()).collect();

        for path in all_paths {
            let src_file = source.get(path);
            let dst_file = dest.get(path);

            let action = match (src_file, dst_file) {
                // 两边都有
                (Some(src), Some(dst)) => {
                    // 跳过目录
                    if src.is_dir && dst.is_dir {
                        continue;
                    }

                    match self.compare_files(src, dst) {
                        FileRelation::Equal | FileRelation::ProbablyEqual => {
                            SyncAction::Skip { path: path.clone() }
                        }
                        FileRelation::SourceNewer => SyncAction::Copy {
                            source_path: path.clone(),
                            dest_path: path.clone(),
                            size: src.size,
                            reverse: false,
                        },
                        FileRelation::DestNewer => {
                            match mode {
                                SyncMode::Bidirectional => {
                                    // 双向同步：目标更新时，从目标同步到源
                                    SyncAction::Copy {
                                        source_path: path.clone(),
                                        dest_path: path.clone(),
                                        size: dst.size,
                                        reverse: true,
                                    }
                                }
                                SyncMode::Mirror | SyncMode::Backup => {
                                    // 镜像/备份：总是用源覆盖目标
                                    SyncAction::Copy {
                                        source_path: path.clone(),
                                        dest_path: path.clone(),
                                        size: src.size,
                                        reverse: false,
                                    }
                                }
                            }
                        }
                        FileRelation::Different => {
                            // 大小不同，根据模式处理
                            match mode {
                                SyncMode::Bidirectional => {
                                    // 双向同步时，大小不同是冲突
                                    SyncAction::Conflict {
                                        path: path.clone(),
                                        source_info: Some(src.clone()),
                                        dest_info: Some(dst.clone()),
                                        conflict_type: ConflictType::BothModified,
                                    }
                                }
                                SyncMode::Mirror | SyncMode::Backup => {
                                    // 镜像/备份：用源覆盖
                                    SyncAction::Copy {
                                        source_path: path.clone(),
                                        dest_path: path.clone(),
                                        size: src.size,
                                        reverse: false,
                                    }
                                }
                            }
                        }
                    }
                }

                // 只有源有
                (Some(src), None) => {
                    if src.is_dir {
                        continue; // 目录会在复制文件时自动创建
                    }
                    SyncAction::Copy {
                        source_path: path.clone(),
                        dest_path: path.clone(),
                        size: src.size,
                        reverse: false,
                    }
                }

                // 只有目标有
                (None, Some(dst)) => {
                    if dst.is_dir {
                        continue;
                    }
                    match mode {
                        SyncMode::Mirror => {
                            // 镜像模式：删除目标中多余的文件
                            SyncAction::Delete {
                                path: path.clone(),
                                from_dest: true,
                            }
                        }
                        SyncMode::Bidirectional => {
                            // 双向同步：从目标复制到源
                            SyncAction::Copy {
                                source_path: path.clone(),
                                dest_path: path.clone(),
                                size: dst.size,
                                reverse: true,
                            }
                        }
                        SyncMode::Backup => {
                            // 备份模式：保留目标中的额外文件
                            SyncAction::Skip { path: path.clone() }
                        }
                    }
                }

                (None, None) => unreachable!(),
            };

            actions.push(action);
        }

        // 按操作类型和路径排序，确保一致性
        actions.sort_by(|a, b| {
            let order_a = match a {
                SyncAction::Copy { .. } => 0,
                SyncAction::Delete { .. } => 2,
                SyncAction::Skip { .. } => 3,
                SyncAction::Conflict { .. } => 1,
            };
            let order_b = match b {
                SyncAction::Copy { .. } => 0,
                SyncAction::Delete { .. } => 2,
                SyncAction::Skip { .. } => 3,
                SyncAction::Conflict { .. } => 1,
            };

            order_a.cmp(&order_b).then_with(|| {
                let path_a = match a {
                    SyncAction::Copy { source_path, .. } => source_path,
                    SyncAction::Delete { path, .. } => path,
                    SyncAction::Skip { path } => path,
                    SyncAction::Conflict { path, .. } => path,
                };
                let path_b = match b {
                    SyncAction::Copy { source_path, .. } => source_path,
                    SyncAction::Delete { path, .. } => path,
                    SyncAction::Skip { path } => path,
                    SyncAction::Conflict { path, .. } => path,
                };
                path_a.cmp(path_b)
            })
        });

        actions
    }

    /// 统计同步动作
    pub fn summarize_actions(actions: &[SyncAction]) -> ActionSummary {
        let mut summary = ActionSummary::default();

        for action in actions {
            match action {
                SyncAction::Copy { size, reverse, .. } => {
                    if *reverse {
                        summary.reverse_copy_count += 1;
                        summary.reverse_copy_bytes += size;
                    } else {
                        summary.copy_count += 1;
                        summary.copy_bytes += size;
                    }
                }
                SyncAction::Delete { .. } => summary.delete_count += 1,
                SyncAction::Skip { .. } => summary.skip_count += 1,
                SyncAction::Conflict { .. } => summary.conflict_count += 1,
            }
        }

        summary
    }
}

impl Default for FileComparator {
    fn default() -> Self {
        Self::new(false)
    }
}

/// 动作统计
#[derive(Debug, Clone, Default)]
pub struct ActionSummary {
    pub copy_count: usize,
    pub copy_bytes: u64,
    pub reverse_copy_count: usize,
    pub reverse_copy_bytes: u64,
    pub delete_count: usize,
    pub skip_count: usize,
    pub conflict_count: usize,
}

impl ActionSummary {
    pub fn total_files(&self) -> usize {
        self.copy_count
            + self.reverse_copy_count
            + self.delete_count
            + self.skip_count
            + self.conflict_count
    }

    pub fn total_transfer_bytes(&self) -> u64 {
        self.copy_bytes + self.reverse_copy_bytes
    }
}
