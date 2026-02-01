// 存储类型
export type StorageType = "local" | "s3" | "webdav";

// 同步模式
export type SyncMode = "bidirectional" | "mirror" | "backup";

// 存储配置
export interface StorageConfig {
  type: StorageType;
  // 本地存储
  path?: string;
  // S3 配置
  bucket?: string;
  region?: string;
  accessKey?: string;
  secretKey?: string;
  endpoint?: string;
  prefix?: string;
  // WebDAV 配置
  webdavEndpoint?: string;
  username?: string;
  password?: string;
  root?: string;
}

// 同步任务
export interface SyncJob {
  id: string;
  name: string;
  sourceConfig: StorageConfig;
  destConfig: StorageConfig;
  syncMode: SyncMode;
  schedule?: string | null;
  enabled: boolean;
  createdAt?: number;
  updatedAt?: number;
}

// 同步进度
export interface SyncProgress {
  jobId: string;
  status:
    | "idle"
    | "scanning"
    | "comparing"
    | "syncing"
    | "completed"
    | "failed"
    | "cancelled"
    | "paused";
  phase: string;
  currentFile: string;
  filesScanned: number;
  filesToSync: number;
  filesCompleted: number;
  filesSkipped: number;
  filesFailed: number;
  bytesTransferred: number;
  bytesTotal: number;
  speed: number;
  eta: number;
  startTime: number;
}

// 同步历史记录
export interface SyncHistoryEntry {
  id: number;
  job_id: string;
  start_time: number;
  end_time: number | null;
  status: string;
  files_scanned: number;
  files_copied: number;
  files_deleted: number | null;
  files_skipped: number | null;
  files_failed: number | null;
  bytes_transferred: number;
  error_message: string | null;
}

// 连接测试结果
export interface TestConnectionResult {
  success: boolean;
  message: string;
  details: string | null;
}
