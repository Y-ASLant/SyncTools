import { type ClassValue, clsx } from "clsx";
import { twMerge } from "tailwind-merge";
import type { StorageType, SyncMode } from "./types";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

// 格式化字节数
export function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B";
  const k = 1024;
  const sizes = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(2))} ${sizes[i]}`;
}

// 格式化时间戳
export function formatTime(timestamp: number): string {
  return new Date(timestamp * 1000).toLocaleString("zh-CN", {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
}

// 存储类型标签
export function getStorageTypeLabel(type: StorageType): string {
  const labels: Record<StorageType, string> = {
    local: "本地",
    s3: "S3",
    webdav: "WebDAV",
  };
  return labels[type] || type;
}

// 同步模式标签
export function getSyncModeLabel(mode: SyncMode): string {
  const labels: Record<SyncMode, string> = {
    mirror: "镜像",
    bidirectional: "双向",
    backup: "备份",
  };
  return labels[mode] || mode;
}

// 同步模式描述
export function getSyncModeDescription(mode: SyncMode): string {
  const descriptions: Record<SyncMode, string> = {
    mirror: "完全同步，删除目标多余文件",
    bidirectional: "双向同步，保留两端更新",
    backup: "仅复制新增和修改的文件",
  };
  return descriptions[mode] || "";
}
