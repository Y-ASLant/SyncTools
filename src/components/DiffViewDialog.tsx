import { useState, useEffect, useMemo, useTransition } from "react";
import {
  X,
  ArrowRight,
  FileText,
  Trash2,
  Copy,
  AlertTriangle,
  CheckCircle,
  Folder,
  ChevronLeft,
  ChevronRight,
} from "lucide-react";
import { cn } from "../lib/utils";
import { useDialog } from "../hooks";

export interface DiffAction {
  type: "copy" | "delete" | "skip" | "conflict";
  path: string;
  size: number;
  reverse: boolean; // true = 从目标到源
  sourceExists: boolean;
  destExists: boolean;
}

export interface DiffResult {
  sourceName: string;
  destName: string;
  sourceFiles: number;
  destFiles: number;
  actions: DiffAction[];
  copyCount: number;
  deleteCount: number;
  skipCount: number;
  conflictCount: number;
  totalBytes: number;
}

interface DiffViewDialogProps {
  isOpen: boolean;
  onClose: () => void;
  diffResult: DiffResult | null;
  onSync: () => void;
}

export function DiffViewDialog({
  isOpen,
  onClose,
  diffResult,
  onSync,
}: DiffViewDialogProps) {
  const [filter, setFilter] = useState<"all" | "copy" | "delete" | "skip">(
    "all",
  );
  const [isPending, startTransition] = useTransition();
  const [currentPage, setCurrentPage] = useState(1);
  const pageSize = 100;

  const { visible, isClosing, handleClose } = useDialog(
    isOpen && !!diffResult,
    onClose
  );

  // 重置页码
  useEffect(() => {
    if (isOpen && diffResult) {
      setCurrentPage(1);
    }
  }, [isOpen, diffResult]);

  // 使用 useTransition 进行非阻塞筛选
  const handleFilterChange = (
    newFilter: "all" | "copy" | "delete" | "skip",
  ) => {
    startTransition(() => {
      setFilter(newFilter);
      setCurrentPage(1);
    });
  };

  // 使用 useMemo 缓存过滤结果
  const filteredActions = useMemo(() => {
    if (!diffResult) return [];
    if (filter === "all") return diffResult.actions;
    return diffResult.actions.filter((action) => action.type === filter);
  }, [diffResult, filter]);

  // 分页计算
  const totalPages = Math.ceil(filteredActions.length / pageSize);
  const startIndex = (currentPage - 1) * pageSize;
  const displayActions = filteredActions.slice(
    startIndex,
    startIndex + pageSize,
  );

  if (!visible || !diffResult) return null;

  const formatBytes = (bytes: number): string => {
    if (bytes === 0) return "0 B";
    const k = 1024;
    const sizes = ["B", "KB", "MB", "GB"];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + " " + sizes[i];
  };

  const getActionIcon = (action: DiffAction) => {
    switch (action.type) {
      case "copy":
        return <Copy className="w-3.5 h-3.5 text-blue-500" />;
      case "delete":
        return <Trash2 className="w-3.5 h-3.5 text-red-500" />;
      case "skip":
        return <CheckCircle className="w-3.5 h-3.5 text-green-500" />;
      case "conflict":
        return <AlertTriangle className="w-3.5 h-3.5 text-yellow-500" />;
    }
  };

  const getActionText = (action: DiffAction) => {
    switch (action.type) {
      case "copy":
        return action.reverse ? "下载" : "上传";
      case "delete":
        return "删除";
      case "skip":
        return "跳过";
      case "conflict":
        return "冲突";
    }
  };

  // 截断中间部分的路径，保留开头和结尾
  const truncatePath = (path: string, maxLength: number = 40) => {
    if (path.length <= maxLength) return path;
    const ellipsis = "...";
    const charsToShow = maxLength - ellipsis.length;
    const frontChars = Math.ceil(charsToShow / 2);
    const backChars = Math.floor(charsToShow / 2);
    return path.slice(0, frontChars) + ellipsis + path.slice(-backChars);
  };

  return (
    <div
      className={`fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/50 ${isClosing ? "dialog-overlay-out" : "dialog-overlay"}`}
    >
      <div
        className={`w-full max-w-3xl bg-white dark:bg-slate-800 rounded shadow-xl max-h-[calc(100vh-80px)] h-[500px] flex flex-col ${isClosing ? "dialog-content-out" : "dialog-content"}`}
      >
        {/* 头部 */}
        <div className="flex items-center justify-between px-4 py-3 border-b border-slate-200 dark:border-slate-700">
          <div className="flex items-center gap-3">
            <h2 className="text-sm font-medium text-slate-900 dark:text-white">
              文件差异分析
            </h2>
            <div className="flex items-center gap-1.5 text-xs text-slate-500">
              <span className="px-1.5 py-0.5 rounded bg-blue-100 dark:bg-blue-900/30 text-blue-600 dark:text-blue-400">
                {diffResult.copyCount} 复制
              </span>
              <span className="px-1.5 py-0.5 rounded bg-red-100 dark:bg-red-900/30 text-red-600 dark:text-red-400">
                {diffResult.deleteCount} 删除
              </span>
              <span className="px-1.5 py-0.5 rounded bg-green-100 dark:bg-green-900/30 text-green-600 dark:text-green-400">
                {diffResult.skipCount} 相同
              </span>
              {diffResult.conflictCount > 0 && (
                <span className="px-1.5 py-0.5 rounded bg-yellow-100 dark:bg-yellow-900/30 text-yellow-600 dark:text-yellow-400">
                  {diffResult.conflictCount} 冲突
                </span>
              )}
            </div>
          </div>
          <button
            onClick={() => handleClose()}
            className="p-1 rounded hover:bg-slate-100 dark:hover:bg-slate-700 transition-colors"
          >
            <X className="w-4 h-4 text-slate-500" />
          </button>
        </div>

        {/* 双列标题 */}
        <div className="grid grid-cols-2 gap-2 px-4 py-2 bg-slate-50 dark:bg-slate-900 border-b border-slate-200 dark:border-slate-700">
          <div className="flex items-center gap-2 min-w-0">
            <Folder className="w-4 h-4 text-blue-500 flex-shrink-0" />
            <span
              className="text-xs font-medium text-slate-700 dark:text-slate-300"
              title={diffResult.sourceName}
            >
              {truncatePath(diffResult.sourceName, 35)}
            </span>
            <span className="text-xs text-slate-500 flex-shrink-0">
              ({diffResult.sourceFiles} 文件)
            </span>
          </div>
          <div className="flex items-center gap-2 min-w-0">
            <Folder className="w-4 h-4 text-green-500 flex-shrink-0" />
            <span
              className="text-xs font-medium text-slate-700 dark:text-slate-300"
              title={diffResult.destName}
            >
              {truncatePath(diffResult.destName, 35)}
            </span>
            <span className="text-xs text-slate-500 flex-shrink-0">
              ({diffResult.destFiles} 文件)
            </span>
          </div>
        </div>

        {/* 筛选器 */}
        <div className="flex items-center gap-1 px-4 py-2 border-b border-slate-200 dark:border-slate-700">
          <span className="text-xs text-slate-500 mr-2">筛选:</span>
          {(["all", "copy", "delete", "skip"] as const).map((f) => (
            <button
              key={f}
              onClick={() => handleFilterChange(f)}
              className={cn(
                "px-2 py-0.5 text-xs rounded transition-colors",
                isPending && "opacity-50",
                filter === f
                  ? "bg-blue-500 text-white"
                  : "bg-slate-100 dark:bg-slate-700 text-slate-600 dark:text-slate-400 hover:bg-slate-200 dark:hover:bg-slate-600",
              )}
            >
              {f === "all"
                ? "全部"
                : f === "copy"
                  ? "复制"
                  : f === "delete"
                    ? "删除"
                    : "相同"}
            </button>
          ))}
        </div>

        {/* 文件列表 */}
        <div className="flex-1 overflow-y-auto scrollable">
          {filteredActions.length === 0 ? (
            <div className="flex items-center justify-center py-12 text-slate-500 text-sm">
              没有符合条件的文件
            </div>
          ) : (
            <div className="divide-y divide-slate-100 dark:divide-slate-700">
              {displayActions.map((action, index) => (
                <div
                  key={index}
                  className="grid grid-cols-[1fr,auto,1fr] gap-2 px-4 py-2 hover:bg-slate-50 dark:hover:bg-slate-700/50"
                >
                  {/* 源文件 */}
                  <div className="flex items-center gap-2 min-w-0">
                    {action.sourceExists ? (
                      <>
                        <FileText className="w-4 h-4 text-slate-400 flex-shrink-0" />
                        <span className="text-xs text-slate-700 dark:text-slate-300 truncate">
                          {action.path}
                        </span>
                        <span className="text-xs text-slate-400 flex-shrink-0">
                          {formatBytes(action.size)}
                        </span>
                      </>
                    ) : (
                      <span className="text-xs text-slate-400 italic">
                        (不存在)
                      </span>
                    )}
                  </div>

                  {/* 操作指示 */}
                  <div className="flex items-center gap-1.5 px-2">
                    {getActionIcon(action)}
                    <ArrowRight
                      className={cn(
                        "w-3.5 h-3.5",
                        action.reverse ? "rotate-180" : "",
                        action.type === "copy"
                          ? "text-blue-500"
                          : action.type === "delete"
                            ? "text-red-500"
                            : "text-slate-300",
                      )}
                    />
                    <span
                      className={cn(
                        "text-xs",
                        action.type === "copy"
                          ? "text-blue-500"
                          : action.type === "delete"
                            ? "text-red-500"
                            : action.type === "skip"
                              ? "text-green-500"
                              : "text-yellow-500",
                      )}
                    >
                      {getActionText(action)}
                    </span>
                  </div>

                  {/* 目标文件 */}
                  <div className="flex items-center gap-2 min-w-0">
                    {action.destExists ? (
                      <>
                        <FileText className="w-4 h-4 text-slate-400 flex-shrink-0" />
                        <span className="text-xs text-slate-700 dark:text-slate-300 truncate">
                          {action.path}
                        </span>
                        {action.type !== "copy" && (
                          <span className="text-xs text-slate-400 flex-shrink-0">
                            {formatBytes(action.size)}
                          </span>
                        )}
                      </>
                    ) : (
                      <span className="text-xs text-slate-400 italic">
                        (不存在)
                      </span>
                    )}
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>

        {/* 分页控件 */}
        {totalPages > 1 && (
          <div className="flex items-center justify-center gap-2 px-4 py-2 border-t border-slate-200 dark:border-slate-700">
            <button
              onClick={() => setCurrentPage((p) => Math.max(1, p - 1))}
              disabled={currentPage === 1}
              className="p-1 rounded hover:bg-slate-100 dark:hover:bg-slate-700 disabled:opacity-30 disabled:cursor-not-allowed"
            >
              <ChevronLeft className="w-4 h-4" />
            </button>
            <span className="text-xs text-slate-600 dark:text-slate-400 min-w-[80px] text-center">
              {currentPage} / {totalPages}
            </span>
            <button
              onClick={() => setCurrentPage((p) => Math.min(totalPages, p + 1))}
              disabled={currentPage === totalPages}
              className="p-1 rounded hover:bg-slate-100 dark:hover:bg-slate-700 disabled:opacity-30 disabled:cursor-not-allowed"
            >
              <ChevronRight className="w-4 h-4" />
            </button>
          </div>
        )}

        {/* 底部操作栏 */}
        <div className="flex items-center justify-between px-4 py-3 border-t border-slate-200 dark:border-slate-700 bg-slate-50 dark:bg-slate-900">
          <div className="text-xs text-slate-500">
            共 {diffResult.actions.length} 项，需传输{" "}
            {formatBytes(diffResult.totalBytes)}
          </div>
          <div className="flex items-center gap-2">
            <button
              onClick={() => handleClose()}
              className="px-3 py-1.5 text-xs rounded border border-slate-200 dark:border-slate-600 text-slate-600 dark:text-slate-400 hover:bg-slate-100 dark:hover:bg-slate-700 transition-colors btn-press"
            >
              关闭
            </button>
            <button
              onClick={onSync}
              disabled={diffResult.copyCount + diffResult.deleteCount === 0}
              className={cn(
                "px-3 py-1.5 text-xs rounded text-white transition-colors btn-press",
                diffResult.copyCount + diffResult.deleteCount === 0
                  ? "bg-slate-400 cursor-not-allowed"
                  : "bg-blue-500 hover:bg-blue-600",
              )}
            >
              开始同步
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
