# SyncTools

高性能文件同步与备份工具，支持本地存储、WebDAV、S3 等多种存储后端。

## 功能特性

- **多存储后端**: 本地文件系统、WebDAV（坚果云、123云盘）、S3 兼容存储（AWS、MinIO、阿里云 OSS、七牛云）
- **灵活同步**: 支持任意存储组合（本地↔本地、本地↔远程、远程↔远程）
- **同步模式**:
  - 双向同步 ↔ 源和目标双向同步
  - 镜像同步 → 源完全复制到目标，删除多余文件
  - 备份同步 → 仅从源复制新增/修改到目标
- **智能缓存**: 远程存储文件列表缓存（可配置 0-2 小时），本地存储直接扫描
- **高性能**: 可配置并行传输数（1-128），增量同步，断点续传
- **冲突处理**: 双向同步时支持手动选择保留源/目标/两者/跳过
- **同步预览**: 同步前差异分析，显示将要执行的操作
- **现代 UI**: 深色/浅色主题，实时进度显示，用时统计

## 技术栈

| 类别 | 技术 |
|------|------|
| 前端 | React + TypeScript + TailwindCSS |
| 后端 | Rust + Tauri 2.0 |
| 数据库 | SQLite |
| 存储抽象 | OpenDAL |

## 系统要求

- Windows 10/11 (64-bit)
- 约 50MB 磁盘空间

## 快速开始

### 开发模式

```bash
npm install
npm run tauri dev
```

### 构建发布版

```bash
npm run tauri build
```

产物位于 `src-tauri/target/release/bundle/`

## 项目结构

```
SyncTools/
├── src/                    # 前端 React
│   ├── components/         # UI 组件
│   ├── hooks/              # 自定义 Hooks
│   └── lib/                # 工具函数、状态管理、类型定义
├── src-tauri/              # 后端 Rust
│   ├── src/
│   │   ├── commands/       # Tauri 命令
│   │   ├── core/           # 同步引擎、缓存、冲突处理
│   │   ├── db/             # 数据库模型
│   │   └── storage/        # 存储后端 (Local/S3/WebDAV)
│   └── migrations/         # 数据库迁移
└── package.json
```

## 数据存储

默认位置: `%APPDATA%/synctools/`（可在设置中修改）

- `synctools.db` - 任务配置、同步历史、文件状态
- `config.json` - 应用配置（日志、缓存 TTL 等）
- `cache/` - 远程存储文件列表缓存
- `app.log` - 应用日志（可配置大小限制）

## 传输机制

- **本地 ↔ 本地**: 直接文件复制
- **本地 ↔ 远程**: 流式上传/下载
- **远程 ↔ 远程**: 数据经本地内存中转（源 → 内存 → 目标）

> 注意: 远程到远程同步时，文件数据会加载到内存中转，大文件建议使用本地中转。

## S3 配置示例

### 七牛云

| 字段 | 值 |
|------|-----|
| Bucket | 空间名称（如 `my-bucket`） |
| Region | 区域代码（如 `cn-north-1`） |
| Endpoint | `http://s3-cn-north-1.qiniucs.com` |
| Access Key | AK |
| Secret Key | SK |

### 阿里云 OSS

| 字段 | 值 |
|------|-----|
| Bucket | Bucket 名称 |
| Region | `oss-cn-hangzhou` 等 |
| Endpoint | `https://oss-cn-hangzhou.aliyuncs.com` |
| Access Key | AccessKey ID |
| Secret Key | AccessKey Secret |

## 设置选项

| 选项 | 说明 | 默认值 |
|------|------|--------|
| 并行传输数 | 同时传输的文件数 | 4 |
| 自动创建目录 | 目标不存在时自动创建 | 开启 |
| 远程缓存过期 | 远程存储文件列表缓存时间 | 30 分钟 |
| 启用日志 | 记录应用日志到文件 | 开启 |
| 日志大小限制 | 单个日志文件最大大小 | 5 MB |

## 许可证

MIT License

## 作者

ASLant
