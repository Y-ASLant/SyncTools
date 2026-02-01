import { useState } from "react";
import { X, FileWarning, Check, Clock, HardDrive } from "lucide-react";
import { cn } from "../lib/utils";
import { useDialog } from "../hooks";

export interface ConflictInfo {
  path: string;
  sourceSize: number;
  sourceTime: number;
  destSize: number;
  destTime: number;
  conflictType:
    | "both_modified"
    | "same_size_different_time"
    | "modified_vs_deleted";
}

export type ConflictResolution =
  | "keep_source"
  | "keep_dest"
  | "keep_both"
  | "skip";

interface ConflictDialogProps {
  isOpen: boolean;
  onClose: () => void;
  conflicts: ConflictInfo[];
  onResolve: (resolutions: Map<string, ConflictResolution>) => void;
}

export function ConflictDialog({
  isOpen,
  onClose,
  conflicts,
  onResolve,
}: ConflictDialogProps) {
  const [resolutions, setResolutions] = useState<
    Map<string, ConflictResolution>
  >(new Map());
  const [defaultResolution, setDefaultResolution] =
    useState<ConflictResolution>("keep_source");
  
  // 使用自定义 Hook，但需要额外处理 conflicts.length 的条件
  const { visible, isClosing, handleClose } = useDialog(
    isOpen && conflicts.length > 0,
    onClose
  );

  if (!visible || conflicts.length === 0) return null;

  const formatSize = (bytes: number) => {
    if (bytes === 0) return "0 B";
    const k = 1024;
    const sizes = ["B", "KB", "MB", "GB"];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + " " + sizes[i];
  };

  const formatTime = (timestamp: number) => {
    return new Date(timestamp * 1000).toLocaleString("zh-CN");
  };

  const getResolution = (path: string): ConflictResolution => {
    return resolutions.get(path) || defaultResolution;
  };

  const setResolution = (path: string, resolution: ConflictResolution) => {
    const newResolutions = new Map(resolutions);
    newResolutions.set(path, resolution);
    setResolutions(newResolutions);
  };

  const applyDefaultToAll = () => {
    const newResolutions = new Map<string, ConflictResolution>();
    conflicts.forEach((c) => newResolutions.set(c.path, defaultResolution));
    setResolutions(newResolutions);
  };

  const handleConfirm = () => {
    // 确保所有冲突都有解决方案
    const finalResolutions = new Map<string, ConflictResolution>();
    conflicts.forEach((c) => {
      finalResolutions.set(c.path, getResolution(c.path));
    });
    handleClose(() => {
      onResolve(finalResolutions);
      onClose();
    });
  };

  const resolutionOptions: {
    value: ConflictResolution;
    label: string;
    description: string;
  }[] = [
    {
      value: "keep_source",
      label: "保留源文件",
      description: "使用源位置的文件覆盖目标",
    },
    {
      value: "keep_dest",
      label: "保留目标文件",
      description: "保持目标位置的文件不变",
    },
    {
      value: "keep_both",
      label: "保留两者",
      description: "重命名冲突文件，保留两个版本",
    },
    { value: "skip", label: "跳过", description: "不处理此冲突" },
  ];

  return (
    <div
      className={`fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/50 ${isClosing ? "dialog-overlay-out" : "dialog-overlay"}`}
    >
      <div
        className={`w-full max-w-3xl bg-white dark:bg-slate-800 rounded-lg shadow-xl max-h-[85vh] flex flex-col ${isClosing ? "dialog-content-out" : "dialog-content"}`}
      >
        {/* 头部 */}
        <div className="flex items-center justify-between p-6 border-b border-slate-200 dark:border-slate-700">
          <div className="flex items-center gap-3">
            <div className="p-2 rounded-lg bg-yellow-100 dark:bg-yellow-900/30">
              <FileWarning className="w-5 h-5 text-yellow-600 dark:text-yellow-400" />
            </div>
            <div>
              <h2 className="text-xl font-semibold text-slate-900 dark:text-white">
                文件冲突
              </h2>
              <p className="text-sm text-slate-500 dark:text-slate-400">
                发现 {conflicts.length} 个冲突需要解决
              </p>
            </div>
          </div>
          <button
            onClick={() => handleClose()}
            className="p-2 rounded-lg hover:bg-slate-100 dark:hover:bg-slate-700 transition-colors"
          >
            <X className="w-5 h-5 text-slate-500" />
          </button>
        </div>

        {/* 默认策略 */}
        <div className="px-6 py-4 border-b border-slate-200 dark:border-slate-700 bg-slate-50 dark:bg-slate-900/50">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium text-slate-700 dark:text-slate-300">
                默认解决策略
              </p>
              <p className="text-xs text-slate-500 dark:text-slate-400">
                应用于未单独设置的冲突
              </p>
            </div>
            <div className="flex items-center gap-2">
              <select
                value={defaultResolution}
                onChange={(e) =>
                  setDefaultResolution(e.target.value as ConflictResolution)
                }
                className="px-3 py-1.5 rounded-lg border border-slate-300 dark:border-slate-600 bg-white dark:bg-slate-800 text-sm"
              >
                {resolutionOptions.map((opt) => (
                  <option key={opt.value} value={opt.value}>
                    {opt.label}
                  </option>
                ))}
              </select>
              <button
                onClick={applyDefaultToAll}
                className="px-3 py-1.5 text-sm bg-blue-500 hover:bg-blue-600 text-white rounded-lg transition-colors"
              >
                应用到全部
              </button>
            </div>
          </div>
        </div>

        {/* 冲突列表 */}
        <div className="flex-1 overflow-y-auto p-6 scrollable">
          <div className="space-y-4">
            {conflicts.map((conflict) => (
              <div
                key={conflict.path}
                className="p-4 rounded-lg border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800"
              >
                <div className="flex items-start justify-between mb-3">
                  <div className="flex-1 min-w-0">
                    <p
                      className="font-medium text-slate-900 dark:text-white truncate"
                      title={conflict.path}
                    >
                      {conflict.path}
                    </p>
                    <p className="text-xs text-slate-500 dark:text-slate-400 mt-1">
                      {conflict.conflictType === "both_modified" &&
                        "两边都有修改"}
                      {conflict.conflictType === "same_size_different_time" &&
                        "大小相同但时间不同"}
                      {conflict.conflictType === "modified_vs_deleted" &&
                        "一边修改一边删除"}
                    </p>
                  </div>
                </div>

                {/* 文件对比 */}
                <div className="grid grid-cols-2 gap-4 mb-4">
                  <div className="p-3 rounded-lg bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800">
                    <p className="text-xs font-medium text-blue-600 dark:text-blue-400 mb-2">
                      源文件
                    </p>
                    <div className="space-y-1 text-sm text-slate-600 dark:text-slate-400">
                      <div className="flex items-center gap-2">
                        <HardDrive className="w-3.5 h-3.5" />
                        <span>{formatSize(conflict.sourceSize)}</span>
                      </div>
                      <div className="flex items-center gap-2">
                        <Clock className="w-3.5 h-3.5" />
                        <span>{formatTime(conflict.sourceTime)}</span>
                      </div>
                    </div>
                  </div>
                  <div className="p-3 rounded-lg bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-800">
                    <p className="text-xs font-medium text-green-600 dark:text-green-400 mb-2">
                      目标文件
                    </p>
                    <div className="space-y-1 text-sm text-slate-600 dark:text-slate-400">
                      <div className="flex items-center gap-2">
                        <HardDrive className="w-3.5 h-3.5" />
                        <span>{formatSize(conflict.destSize)}</span>
                      </div>
                      <div className="flex items-center gap-2">
                        <Clock className="w-3.5 h-3.5" />
                        <span>{formatTime(conflict.destTime)}</span>
                      </div>
                    </div>
                  </div>
                </div>

                {/* 解决方案选择 */}
                <div className="flex flex-wrap gap-2">
                  {resolutionOptions.map((opt) => (
                    <button
                      key={opt.value}
                      onClick={() => setResolution(conflict.path, opt.value)}
                      className={cn(
                        "px-3 py-1.5 text-sm rounded-lg border transition-colors",
                        getResolution(conflict.path) === opt.value
                          ? "border-blue-500 bg-blue-50 dark:bg-blue-900/30 text-blue-700 dark:text-blue-300"
                          : "border-slate-200 dark:border-slate-600 hover:border-slate-300 dark:hover:border-slate-500",
                      )}
                    >
                      {opt.label}
                    </button>
                  ))}
                </div>
              </div>
            ))}
          </div>
        </div>

        {/* 底部操作 */}
        <div className="flex items-center justify-between p-6 border-t border-slate-200 dark:border-slate-700">
          <p className="text-sm text-slate-500 dark:text-slate-400">
            已设置 {resolutions.size} / {conflicts.length} 个冲突
          </p>
          <div className="flex gap-3">
            <button
              onClick={() => handleClose()}
              className="px-4 py-2 rounded-lg border border-slate-300 dark:border-slate-600 text-slate-700 dark:text-slate-300 hover:bg-slate-50 dark:hover:bg-slate-700 transition-colors"
            >
              取消
            </button>
            <button
              onClick={handleConfirm}
              className="px-4 py-2 rounded-lg bg-blue-500 hover:bg-blue-600 text-white transition-colors flex items-center gap-2"
            >
              <Check className="w-4 h-4" />
              确认解决
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
