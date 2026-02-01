# SyncTools

高性能文件同步与备份工具，支持本地存储、WebDAV、S3 等多种存储后端。

## 功能特性

- **多存储后端支持**
  - 本地文件系统
  - WebDAV（支持坚果云、123云盘等）
  - S3 兼容存储（AWS S3、MinIO、阿里云 OSS 等）

- **多种同步模式**
  - 双向同步：保持两端文件一致
  - 镜像同步：目标端完全复制源端
  - 仅备份：只从源端复制到目标端

- **智能同步**
  - 基于文件哈希的增量同步
  - 远程文件列表缓存，加速后续同步
  - 可配置的并行传输数（最高 128）

- **用户友好**
  - 现代化 UI 界面
  - 深色/浅色主题切换
  - 同步前差异预览
  - 实时进度显示

## 技术栈

- **前端**: React + TypeScript + TailwindCSS
- **后端**: Rust + Tauri 2.0
- **数据库**: SQLite
- **存储抽象**: OpenDAL

## 系统要求

- Windows 10/11 (64-bit)
- 约 50MB 磁盘空间

## 开发环境

### 依赖

- Node.js 18+
- Rust 1.70+
- pnpm 或 npm

### 安装依赖

```bash
# 安装前端依赖
npm install

# Rust 依赖会在首次构建时自动安装
```

### 开发模式

```bash
npm run tauri dev
```

### 构建发布版

```bash
npm run tauri build
```

构建产物位于 `src-tauri/target/release/bundle/`

## 项目结构

```
SyncTools/
├── src/                    # 前端源码
│   ├── components/         # React 组件
│   ├── lib/               # 工具函数和状态管理
│   └── App.tsx            # 主应用组件
├── src-tauri/             # Rust 后端
│   ├── src/
│   │   ├── commands/      # Tauri 命令
│   │   ├── core/          # 核心同步逻辑
│   │   ├── db/            # 数据库模型
│   │   └── storage/       # 存储后端实现
│   └── migrations/        # 数据库迁移
└── package.json
```

## 数据存储

应用数据默认存储在：
- Windows: `%APPDATA%/synctools/`

包含：
- `synctools.db` - SQLite 数据库（任务配置、同步历史、文件状态）
- `config.json` - 应用配置

可在设置中修改数据存储位置。

## 许可证

MIT License

## 作者

ASLant
