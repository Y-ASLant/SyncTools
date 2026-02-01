-- 同步任务表
CREATE TABLE IF NOT EXISTS sync_jobs (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    source_type TEXT NOT NULL,
    source_config TEXT NOT NULL,
    dest_type TEXT NOT NULL,
    dest_config TEXT NOT NULL,
    sync_mode TEXT NOT NULL,
    schedule TEXT,
    enabled BOOLEAN DEFAULT 1 NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

-- 文件状态表（用于快速比较）
CREATE TABLE IF NOT EXISTS file_states (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    job_id TEXT NOT NULL,
    file_path TEXT NOT NULL,
    file_size INTEGER NOT NULL,
    modified_time INTEGER NOT NULL,
    checksum TEXT,
    last_sync_time INTEGER,
    FOREIGN KEY (job_id) REFERENCES sync_jobs(id) ON DELETE CASCADE,
    UNIQUE(job_id, file_path)
);

-- 传输状态表（断点续传）
CREATE TABLE IF NOT EXISTS transfer_states (
    id TEXT PRIMARY KEY,
    job_id TEXT NOT NULL,
    file_path TEXT NOT NULL,
    total_size INTEGER NOT NULL,
    transferred_size INTEGER DEFAULT 0,
    upload_id TEXT,
    parts_completed TEXT,
    status TEXT NOT NULL,
    started_at INTEGER,
    updated_at INTEGER,
    FOREIGN KEY (job_id) REFERENCES sync_jobs(id) ON DELETE CASCADE
);

-- 同步日志表
CREATE TABLE IF NOT EXISTS sync_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    job_id TEXT NOT NULL,
    start_time INTEGER NOT NULL,
    end_time INTEGER,
    status TEXT NOT NULL,
    files_scanned INTEGER DEFAULT 0,
    files_copied INTEGER DEFAULT 0,
    files_deleted INTEGER DEFAULT 0,
    bytes_transferred INTEGER DEFAULT 0,
    error_message TEXT,
    FOREIGN KEY (job_id) REFERENCES sync_jobs(id) ON DELETE CASCADE
);

-- 冲突记录表
CREATE TABLE IF NOT EXISTS conflicts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    job_id TEXT NOT NULL,
    file_path TEXT NOT NULL,
    conflict_type TEXT NOT NULL,
    resolution TEXT,
    source_time INTEGER,
    dest_time INTEGER,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (job_id) REFERENCES sync_jobs(id) ON DELETE CASCADE
);

-- 索引
CREATE INDEX IF NOT EXISTS idx_file_states_job ON file_states(job_id);
CREATE INDEX IF NOT EXISTS idx_file_states_path ON file_states(job_id, file_path);
CREATE INDEX IF NOT EXISTS idx_transfer_states_job ON transfer_states(job_id);
CREATE INDEX IF NOT EXISTS idx_transfer_states_upload ON transfer_states(upload_id);
CREATE INDEX IF NOT EXISTS idx_sync_logs_job ON sync_logs(job_id);
CREATE INDEX IF NOT EXISTS idx_conflicts_job ON conflicts(job_id);
CREATE INDEX IF NOT EXISTS idx_sync_logs_time ON sync_logs(start_time DESC);
