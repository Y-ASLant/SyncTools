import { useEffect, useState } from "react";
import { X, Clock, CheckCircle, XCircle, AlertCircle } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import type { SyncHistoryEntry } from "../lib/types";

interface HistoryPanelProps {
  isOpen: boolean;
  onClose: () => void;
  jobId: string;
  jobName: string;
}

export function HistoryPanel({
  isOpen,
  onClose,
  jobId,
  jobName,
}: HistoryPanelProps) {
  const [history, setHistory] = useState<SyncHistoryEntry[]>([]);
  const [loading, setLoading] = useState(false);
  const [visible, setVisible] = useState(false);
  const [isClosing, setIsClosing] = useState(false);

  useEffect(() => {
    if (isOpen) {
      setVisible(true);
      setIsClosing(false);
      loadHistory();
    }
  }, [isOpen, jobId]);

  const handleClose = () => {
    setIsClosing(true);
    setTimeout(() => {
      setVisible(false);
      onClose();
    }, 100);
  };

  const loadHistory = async () => {
    setLoading(true);
    try {
      const result = await invoke<SyncHistoryEntry[]>("get_sync_history", {
        jobId,
        limit: 50,
      });
      setHistory(result);
    } catch (error) {
      console.error("加载历史失败:", error);
    } finally {
      setLoading(false);
    }
  };

  const formatTime = (timestamp: number) => {
    return new Date(timestamp * 1000).toLocaleString("zh-CN");
  };

  const formatDuration = (start: number, end: number | null) => {
    if (!end) return "-";
    const duration = Math.floor((end - start) / 60);
    if (duration < 60) return `${duration}分钟`;
    return `${Math.floor(duration / 60)}小时${duration % 60}分钟`;
  };

  const formatBytes = (bytes: number) => {
    if (bytes === 0) return "0 B";
    const k = 1024;
    const sizes = ["B", "KB", "MB", "GB"];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + " " + sizes[i];
  };

  const getStatusIcon = (status: string) => {
    switch (status) {
      case "completed":
        return <CheckCircle className="w-4 h-4 text-green-500" />;
      case "failed":
        return <XCircle className="w-4 h-4 text-red-500" />;
      case "cancelled":
        return <AlertCircle className="w-4 h-4 text-yellow-500" />;
      default:
        return <Clock className="w-4 h-4 text-blue-500" />;
    }
  };

  const getStatusText = (status: string) => {
    const map: Record<string, string> = {
      completed: "成功",
      failed: "失败",
      cancelled: "已取消",
      running: "进行中",
    };
    return map[status] || status;
  };

  if (!visible) return null;

  return (
    <div
      className={`fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/50 ${isClosing ? "dialog-overlay-out" : "dialog-overlay"}`}
    >
      <div
        className={`w-full max-w-2xl bg-white dark:bg-slate-800 rounded shadow-xl max-h-[80vh] flex flex-col ${isClosing ? "dialog-content-out" : "dialog-content"}`}
      >
        {/* 头部 */}
        <div className="flex items-center justify-between px-4 py-3 border-b border-slate-200 dark:border-slate-700">
          <div>
            <h2 className="text-sm font-medium text-slate-900 dark:text-white">
              同步历史
            </h2>
            <p className="text-xs text-slate-500 dark:text-slate-400">
              {jobName}
            </p>
          </div>
          <button
            onClick={handleClose}
            className="p-1 rounded hover:bg-slate-100 dark:hover:bg-slate-700 transition-colors"
          >
            <X className="w-4 h-4 text-slate-500" />
          </button>
        </div>

        {/* 内容 */}
        <div className="flex-1 overflow-y-auto p-4 scrollable">
          {loading ? (
            <div className="flex items-center justify-center py-8">
              <div className="w-5 h-5 border-2 border-blue-500 border-t-transparent rounded-full animate-spin" />
            </div>
          ) : history.length === 0 ? (
            <div className="text-center py-8">
              <Clock className="w-8 h-8 text-slate-300 dark:text-slate-600 mx-auto mb-2" />
              <p className="text-xs text-slate-500 dark:text-slate-400">
                暂无同步记录
              </p>
            </div>
          ) : (
            <div className="space-y-2">
              {history.map((entry) => (
                <div
                  key={entry.id}
                  className="p-3 rounded border border-slate-200 dark:border-slate-700 hover:bg-slate-50 dark:hover:bg-slate-800/50 transition-colors"
                >
                  <div className="flex items-center justify-between mb-2">
                    <div className="flex items-center gap-1.5">
                      {getStatusIcon(entry.status)}
                      <span className="text-sm font-medium text-slate-900 dark:text-white">
                        {getStatusText(entry.status)}
                      </span>
                    </div>
                    <span className="text-xs text-slate-500">
                      {formatTime(entry.start_time)}
                    </span>
                  </div>

                  <div className="grid grid-cols-4 gap-3 text-xs">
                    <div>
                      <p className="text-slate-500">扫描</p>
                      <p className="font-medium text-slate-900 dark:text-white">
                        {entry.files_scanned}
                      </p>
                    </div>
                    <div>
                      <p className="text-slate-500">复制</p>
                      <p className="font-medium text-slate-900 dark:text-white">
                        {entry.files_copied}
                      </p>
                    </div>
                    {entry.files_deleted !== null && (
                      <div>
                        <p className="text-slate-500">删除</p>
                        <p className="font-medium text-slate-900 dark:text-white">
                          {entry.files_deleted}
                        </p>
                      </div>
                    )}
                    {entry.files_failed !== null && entry.files_failed > 0 && (
                      <div>
                        <p className="text-slate-500">失败</p>
                        <p className="font-medium text-red-500">
                          {entry.files_failed}
                        </p>
                      </div>
                    )}
                  </div>

                  <div className="mt-2 pt-2 border-t border-slate-100 dark:border-slate-800 flex items-center justify-between text-xs text-slate-500">
                    <div className="flex items-center gap-3">
                      <span>{formatBytes(entry.bytes_transferred)}</span>
                      <span>
                        {formatDuration(entry.start_time, entry.end_time)}
                      </span>
                    </div>
                    {entry.error_message && (
                      <span className="text-red-500 truncate max-w-xs">
                        {entry.error_message}
                      </span>
                    )}
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
