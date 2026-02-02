import { useEffect, useState, useRef } from "react";
import { useSyncStore } from "./lib/store";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  RefreshCw,
  Plus,
  Trash2,
  Play,
  Clock,
  StopCircle,
  Pencil,
  ArrowRight,
  ArrowLeftRight,
  RotateCcw,
} from "lucide-react";
import { cn, getStorageTypeLabel, getSyncModeLabel } from "./lib/utils";
import { NEW_JOB_THRESHOLD_SECONDS } from "./lib/constants";
import {
  CreateJobDialog,
  SettingsDialog,
  HistoryPanel,
  ToastContainer,
  useToast,
  TitleBar,
  ConfirmDialog,
  DiffViewDialog,
  ConflictDialog,
  AnimatedBytes,
  AnimatedSpeed,
} from "./components";
import type { SyncProgress, SyncJob } from "./lib/types";
import type { DiffResult, ConflictInfo, ConflictResolution } from "./components";

function App() {
  const {
    jobs,
    progress,
    setProgress,
    removeJob,
    clearProgress,
  } = useSyncStore();
  const [isDialogOpen, setIsDialogOpen] = useState(false);
  const [isSettingsOpen, setIsSettingsOpen] = useState(false);
  const [editingJob, setEditingJob] = useState<SyncJob | null>(null);
  const [deletingJob, setDeletingJob] = useState<{
    id: string;
    name: string;
  } | null>(null);
  const [historyJob, setHistoryJob] = useState<{
    id: string;
    name: string;
  } | null>(null);
  const [diffResult, setDiffResult] = useState<DiffResult | null>(null);
  const [analyzingJobs, setAnalyzingJobs] = useState<Set<string>>(new Set());
  const [diffJobId, setDiffJobId] = useState<string | null>(null);
  const analyzeAbortRef = useRef<Set<string>>(new Set());
  const [filterMode, setFilterMode] = useState<
    "all" | "bidirectional" | "mirror" | "backup"
  >("all");
  // 冲突处理状态
  const [conflicts, setConflicts] = useState<ConflictInfo[]>([]);
  const [conflictJobId, setConflictJobId] = useState<string | null>(null);
  const { toasts, closeToast, success, error: showError, info } = useToast();

  // 禁用浏览器默认行为（右键菜单、快捷键）
  useEffect(() => {
    // 禁用右键菜单
    const handleContextMenu = (e: MouseEvent) => {
      e.preventDefault();
    };

    // 禁用浏览器快捷键
    const handleKeyDown = (e: KeyboardEvent) => {
      // Ctrl+R / Cmd+R - 刷新
      if ((e.ctrlKey || e.metaKey) && e.key === 'r') {
        e.preventDefault();
      }
      // Ctrl+P / Cmd+P - 打印
      if ((e.ctrlKey || e.metaKey) && e.key === 'p') {
        e.preventDefault();
      }
      // Ctrl+S / Cmd+S - 保存
      if ((e.ctrlKey || e.metaKey) && e.key === 's') {
        e.preventDefault();
      }
      // Ctrl+Shift+I / Cmd+Option+I - 开发者工具
      if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key === 'I') {
        e.preventDefault();
      }
      // Ctrl+Shift+J / Cmd+Option+J - 控制台
      if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key === 'J') {
        e.preventDefault();
      }
      // Ctrl+U / Cmd+U - 查看源代码
      if ((e.ctrlKey || e.metaKey) && e.key === 'u') {
        e.preventDefault();
      }
      // F5 - 刷新
      if (e.key === 'F5') {
        e.preventDefault();
      }
      // F12 - 开发者工具
      if (e.key === 'F12') {
        e.preventDefault();
      }
    };

    document.addEventListener('contextmenu', handleContextMenu);
    document.addEventListener('keydown', handleKeyDown);

    return () => {
      document.removeEventListener('contextmenu', handleContextMenu);
      document.removeEventListener('keydown', handleKeyDown);
    };
  }, []);

  // 初始化：加载任务列表和设置事件监听
  useEffect(() => {
    let mounted = true;

    // 加载任务列表
    const loadJobs = async () => {
      try {
        const loadedJobs = await invoke<SyncJob[]>("get_jobs");
        if (mounted && loadedJobs) {
          const store = useSyncStore.getState();
          // 清空并重新加载任务
          store.jobs.forEach((job) => store.removeJob(job.id));
          loadedJobs.forEach((job) => store.addJob(job));
        }
      } catch (err) {
        console.error("加载任务失败:", err);
        // 只在组件仍然挂载时显示错误
        if (mounted) {
          // 延迟显示错误，避免在初始化时出现问题
          setTimeout(() => {
            if (mounted) {
              showError("加载任务失败: " + err);
            }
          }, 100);
        }
      }
    };

    loadJobs();

    // 监听同步进度事件
    const unlistenProgress = listen<SyncProgress>("sync-progress", (event) => {
      if (mounted) {
        setProgress(event.payload.jobId, event.payload);
      }
    });

    // 监听同步完成事件
    interface SyncCompletePayload {
      job_id: string;
      result?: {
        Ok?: {
          status: string;
          errors?: string[];
          filesFailed?: number;
        };
        Err?: string;
      };
    }
    const unlistenComplete = listen<SyncCompletePayload>(
      "sync-complete",
      (event) => {
        if (!mounted) return;
        const { job_id, result } = event.payload;
        const store = useSyncStore.getState();
        const job = store.jobs.find((j) => j.id === job_id);
        const jobProgress = store.progress[job_id];

        // 尝试获取错误信息
        const errors = result?.Ok?.errors || [];
        const firstError = errors.length > 0 ? errors[0] : null;

        if (
          jobProgress?.status === "completed" &&
          (!errors || errors.length === 0)
        ) {
          success("同步完成", `${job?.name || "任务"} 已成功完成`);
        } else if (
          jobProgress?.status === "failed" ||
          (errors && errors.length > 0)
        ) {
          const errorMsg =
            firstError || `${job?.name || "任务"} 同步过程中出现错误`;
          showError("同步失败", errorMsg);
        }
      },
    );

    return () => {
      mounted = false;
      unlistenProgress.then((fn) => fn());
      unlistenComplete.then((fn) => fn());
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const loadJobs = async () => {
    try {
      const loadedJobs = await invoke<SyncJob[]>("get_jobs");
      if (loadedJobs) {
        const store = useSyncStore.getState();
        store.jobs.forEach((job) => store.removeJob(job.id));
        loadedJobs.forEach((job) => store.addJob(job));
      }
    } catch (err) {
      console.error("加载任务失败:", err);
      showError("加载失败", String(err));
    }
  };

  const handleStartSync = async (jobId: string) => {
    const job = jobs.find((j) => j.id === jobId);
    const autoCreateDir = localStorage.getItem("auto-create-dir") !== "false"; // 默认开启
    const maxConcurrent =
      parseInt(localStorage.getItem("max-concurrent") || "4") || 4;
    try {
      await invoke("start_sync", { jobId, autoCreateDir, maxConcurrent });
      info("开始同步", `正在同步 ${job?.name || "任务"}...`);
    } catch (err) {
      console.error("启动同步失败:", err);
      showError("启动失败", String(err));
    }
  };

  const handleCancelSync = async (jobId: string) => {
    try {
      await invoke("cancel_sync", { jobId });
      clearProgress(jobId);
      info("已取消", "同步任务已取消");
    } catch (err) {
      console.error("取消同步失败:", err);
      showError("取消失败", String(err));
    }
  };

  const handleDeleteJob = async (jobId: string) => {
    try {
      await invoke("delete_job", { id: jobId });
      removeJob(jobId);
      setDeletingJob(null);
      success("已删除", "同步任务已删除");
    } catch (err) {
      console.error("删除任务失败:", err);
      showError("删除失败", String(err));
    }
  };

  const handleAnalyzeJob = async (jobId: string, forceRefresh: boolean = false) => {
    // 添加到正在分析的任务集合
    setAnalyzingJobs(prev => new Set(prev).add(jobId));
    analyzeAbortRef.current.delete(jobId);
    try {
      const result = await invoke<DiffResult>("analyze_job", { jobId, forceRefresh });
      // 如果已取消，忽略结果
      if (analyzeAbortRef.current.has(jobId)) {
        info("已取消", "分析操作已取消");
        return;
      }
      setDiffResult(result);
      setDiffJobId(jobId);
      if (forceRefresh) {
        success("刷新完成", "已重新扫描文件列表");
      }
    } catch (err) {
      if (!analyzeAbortRef.current.has(jobId)) {
        console.error("分析任务失败:", err);
        showError("分析失败", String(err));
      }
    } finally {
      // 从正在分析的任务集合中移除
      setAnalyzingJobs(prev => {
        const newSet = new Set(prev);
        newSet.delete(jobId);
        return newSet;
      });
      analyzeAbortRef.current.delete(jobId);
    }
  };

  const handleCancelAnalyze = async (jobId: string) => {
    try {
      await invoke("cancel_analyze", { jobId });
    } catch (err) {
      console.error("取消分析失败:", err);
    }
    analyzeAbortRef.current.add(jobId);
    setAnalyzingJobs(prev => {
      const newSet = new Set(prev);
      newSet.delete(jobId);
      return newSet;
    });
  };

  const handleSyncFromDiff = () => {
    if (!diffJobId || !diffResult) return;
    
    // 检查是否有冲突
    if (diffResult.conflictCount > 0) {
      // 从 actions 中提取冲突信息
      const conflictActions = diffResult.actions.filter(a => a.type === "conflict");
      const conflictInfos: ConflictInfo[] = conflictActions.map(action => ({
        path: action.path,
        sourceSize: action.sourceExists ? action.size : 0,
        sourceTime: Date.now() / 1000, // 后端应该返回实际时间
        destSize: action.destExists ? action.size : 0,
        destTime: Date.now() / 1000,
        conflictType: "both_modified" as const,
      }));
      
      // 显示冲突对话框
      setConflicts(conflictInfos);
      setConflictJobId(diffJobId);
      // 先关闭 diff 对话框
      setDiffResult(null);
      setDiffJobId(null);
      return;
    }
    
    const jobId = diffJobId;
    // 先关闭弹窗
    setDiffResult(null);
    setDiffJobId(null);
    // 然后启动同步
    handleStartSync(jobId);
  };

  // 处理冲突解决
  const handleConflictResolve = async (resolutions: Map<string, ConflictResolution>) => {
    if (!conflictJobId) return;
    
    // 将解决方案转换为后端期望的格式
    const resolutionMap: Record<string, string> = {};
    resolutions.forEach((value, key) => {
      resolutionMap[key] = value;
    });
    
    // 关闭冲突对话框
    setConflicts([]);
    const jobId = conflictJobId;
    setConflictJobId(null);
    
    // 启动同步，传递冲突解决方案
    try {
      await invoke("start_sync", { 
        jobId,
        conflictResolutions: resolutionMap 
      });
    } catch (err) {
      console.error("同步失败:", err);
      showError("同步失败", String(err));
    }
  };

  return (
    <div
      className={cn(
        "h-screen flex flex-col bg-slate-50 dark:bg-slate-950",
        "transition-colors duration-200",
      )}
    >
      {/* 自定义标题栏 */}
      <TitleBar
        onOpenSettings={() => setIsSettingsOpen(true)}
      />

      {/* 主内容 */}
      <main className="flex-1 overflow-auto p-4 scrollable">
        {/* 标题栏 */}
        <div className="flex items-center justify-between mb-3">
          <div className="flex items-center gap-3">
            <div className="flex items-center gap-2">
              <h1 className="text-sm font-medium text-slate-700 dark:text-slate-300">
                同步任务
              </h1>
              {jobs.length > 0 && (
                <span className="px-1.5 py-0.5 rounded-full text-xs bg-blue-100 dark:bg-blue-900/30 text-blue-600 dark:text-blue-400">
                  {jobs.length}
                </span>
              )}
            </div>
            {/* Segment 筛选器 */}
            {jobs.length > 0 && (
              <div className="flex items-center p-0.5 rounded-lg bg-slate-100 dark:bg-slate-800">
                {[
                  { key: "all", label: "全部" },
                  { key: "bidirectional", label: "双向" },
                  { key: "mirror", label: "镜像" },
                  { key: "backup", label: "备份" },
                ].map((item) => (
                  <button
                    key={item.key}
                    onClick={() => setFilterMode(item.key as typeof filterMode)}
                    className={cn(
                      "px-2.5 py-1 text-xs font-medium rounded-md transition-all",
                      filterMode === item.key
                        ? "bg-white dark:bg-slate-700 text-slate-900 dark:text-white shadow-sm"
                        : "text-slate-500 dark:text-slate-400 hover:text-slate-700 dark:hover:text-slate-300",
                    )}
                  >
                    {item.label}
                  </button>
                ))}
              </div>
            )}
          </div>
          <button
            onClick={() => setIsDialogOpen(true)}
            className="flex items-center gap-1.5 px-3 py-1.5 bg-blue-500 hover:bg-blue-600 text-white text-xs font-medium rounded-md shadow-sm transition-colors btn-press"
          >
            <Plus className="w-3.5 h-3.5" />
            新建任务
          </button>
        </div>

        {/* 任务列表 */}
        <div className="space-y-2">
          {(() => {
            const filteredJobs = jobs.filter((job) => {
              if (filterMode === "all") return true;
              return job.syncMode === filterMode;
            });

            if (jobs.length === 0) {
              return (
                <div className="text-center py-20">
                  <div className="w-12 h-12 rounded bg-slate-100 dark:bg-slate-800 flex items-center justify-center mx-auto mb-3">
                    <RefreshCw className="w-6 h-6 text-slate-400" />
                  </div>
                  <h3 className="text-sm font-medium text-slate-700 dark:text-slate-300 mb-1">
                    还没有同步任务
                  </h3>
                  <p className="text-xs text-slate-500 dark:text-slate-500 mb-4">
                    点击右上角"新建任务"开始
                  </p>
                </div>
              );
            }

            if (filteredJobs.length === 0) {
              return (
                <div className="text-center py-12">
                  <p className="text-sm text-slate-500 dark:text-slate-400">
                    没有符合筛选条件的任务
                  </p>
                </div>
              );
            }

            return filteredJobs.map((job) => {
              const jobProgress = progress[job.id];
              const isSyncing =
                jobProgress?.status === "syncing" ||
                jobProgress?.status === "scanning" ||
                jobProgress?.status === "comparing";
              const progressPercent =
                jobProgress?.filesToSync && jobProgress.filesToSync > 0
                  ? Math.round(
                      (jobProgress.filesCompleted / jobProgress.filesToSync) *
                        100,
                    )
                  : 0;

              return (
                <div
                  key={job.id}
                  className={cn(
                    "bg-white dark:bg-slate-900 rounded border card-hover",
                    isSyncing
                      ? "border-blue-400 dark:border-blue-600"
                      : "border-slate-200 dark:border-slate-800 hover:border-slate-300 dark:hover:border-slate-700",
                  )}
                >
                  <div className="p-3">
                    <div className="flex items-center justify-between">
                      <div className="flex-1 min-w-0">
                        <div className="flex items-center gap-2">
                          <h3 className="text-sm font-medium text-slate-900 dark:text-white truncate">
                            {job.name}
                          </h3>
                          {/* 新创建的任务显示 New 标签 */}
                          {job.createdAt && (Date.now() / 1000 - job.createdAt) < NEW_JOB_THRESHOLD_SECONDS && (
                            <span className="px-1.5 py-0.5 rounded text-xs bg-green-100 dark:bg-green-900/30 text-green-600 dark:text-green-400 font-medium">
                              New
                            </span>
                          )}
                          {!job.enabled && (
                            <span className="px-1.5 py-0.5 rounded text-xs bg-slate-100 dark:bg-slate-800 text-slate-500">
                              禁用
                            </span>
                          )}
                        </div>
                        <div className="flex items-center gap-3 mt-1 text-xs text-slate-500 dark:text-slate-500">
                          <span className="flex items-center gap-1">
                            <span className="w-1.5 h-1.5 rounded-full bg-blue-500" />
                            {getStorageTypeLabel(job.sourceConfig.type)}
                          </span>
                          {job.syncMode === "bidirectional" ? (
                            <ArrowLeftRight className="w-3 h-3 text-slate-400" />
                          ) : (
                            <ArrowRight className="w-3 h-3 text-slate-400" />
                          )}
                          <span className="flex items-center gap-1">
                            {getStorageTypeLabel(job.destConfig.type)}
                            <span className="w-1.5 h-1.5 rounded-full bg-green-500" />
                          </span>
                          <span className="px-1.5 py-0.5 rounded bg-slate-100 dark:bg-slate-800">
                            {getSyncModeLabel(job.syncMode)}
                          </span>
                        </div>
                      </div>
                      <div className="flex items-center gap-1 ml-3">
                        {isSyncing ? (
                          <button
                            onClick={() => handleCancelSync(job.id)}
                            className="flex items-center gap-1 px-2 py-1 rounded text-xs bg-red-500 hover:bg-red-600 text-white transition-colors"
                          >
                            <StopCircle className="w-3 h-3" />
                            取消
                          </button>
                        ) : (
                          <>
                            <button
                              onClick={
                                analyzingJobs.has(job.id)
                                  ? () => handleCancelAnalyze(job.id)
                                  : () => handleAnalyzeJob(job.id)
                              }
                              disabled={!job.enabled && !analyzingJobs.has(job.id)}
                              className={cn(
                                "flex items-center justify-center gap-1 px-2 py-1 rounded text-xs transition-colors btn-press",
                                analyzingJobs.has(job.id)
                                  ? "bg-amber-100 dark:bg-amber-900/30 text-amber-600 dark:text-amber-400 hover:bg-red-100 dark:hover:bg-red-900/30 hover:text-red-600 dark:hover:text-red-400"
                                  : !job.enabled
                                    ? "bg-slate-100 dark:bg-slate-800 text-slate-400 cursor-not-allowed"
                                    : "bg-slate-100 dark:bg-slate-700 text-slate-600 dark:text-slate-300 hover:bg-slate-200 dark:hover:bg-slate-600",
                              )}
                              title={
                                analyzingJobs.has(job.id)
                                  ? "点击取消"
                                  : "分析差异"
                              }
                            >
                              {analyzingJobs.has(job.id) ? (
                                <>
                                  <RefreshCw className="w-3 h-3 animate-spin" />
                                  取消
                                </>
                              ) : (
                                <>
                                  <Play className="w-3 h-3" />
                                  分析
                                </>
                              )}
                            </button>
                            <button
                              onClick={() => handleAnalyzeJob(job.id, true)}
                              disabled={!job.enabled || analyzingJobs.has(job.id)}
                              className={cn(
                                "p-1 rounded transition-colors",
                                !job.enabled || analyzingJobs.has(job.id)
                                  ? "text-slate-300 dark:text-slate-700 cursor-not-allowed"
                                  : "text-slate-400 hover:text-blue-500 hover:bg-slate-100 dark:hover:bg-slate-800",
                              )}
                              title="强制刷新（重新扫描文件）"
                            >
                              <RotateCcw className="w-3.5 h-3.5" />
                            </button>
                          </>
                        )}
                        <button
                          onClick={() => setEditingJob(job)}
                          disabled={isSyncing}
                          className={cn(
                            "p-1 rounded transition-colors",
                            isSyncing
                              ? "text-slate-300 dark:text-slate-700 cursor-not-allowed"
                              : "text-slate-400 hover:text-blue-500 hover:bg-slate-100 dark:hover:bg-slate-800",
                          )}
                          title="编辑任务"
                        >
                          <Pencil className="w-3.5 h-3.5" />
                        </button>
                        <button
                          onClick={() =>
                            setHistoryJob({ id: job.id, name: job.name })
                          }
                          className="p-1 rounded text-slate-400 hover:text-blue-500 hover:bg-slate-100 dark:hover:bg-slate-800 transition-colors"
                          title="查看历史"
                        >
                          <Clock className="w-3.5 h-3.5" />
                        </button>
                        <button
                          onClick={() =>
                            setDeletingJob({ id: job.id, name: job.name })
                          }
                          disabled={isSyncing}
                          className={cn(
                            "p-1 rounded transition-colors",
                            isSyncing
                              ? "text-slate-300 dark:text-slate-700 cursor-not-allowed"
                              : "text-slate-400 hover:text-red-500 hover:bg-slate-100 dark:hover:bg-slate-800",
                          )}
                          title="删除任务"
                        >
                          <Trash2 className="w-3.5 h-3.5" />
                        </button>
                      </div>
                    </div>

                    {/* 进度条 */}
                    {jobProgress && isSyncing && (
                      <div className="mt-3 pt-2 border-t border-slate-100 dark:border-slate-800">
                        <div className="flex items-center justify-between text-xs mb-1">
                          <span className="text-slate-500">
                            {jobProgress.phase || "准备中..."}
                          </span>
                          <span className="text-slate-700 dark:text-slate-300">
                            {jobProgress.filesCompleted}/
                            {jobProgress.filesToSync || "?"}
                            {progressPercent > 0 && ` (${progressPercent}%)`}
                          </span>
                        </div>
                        <div className="h-1 bg-slate-100 dark:bg-slate-800 rounded overflow-hidden">
                          <div
                            className="h-full bg-blue-500 transition-all duration-300"
                            style={{ width: `${progressPercent}%` }}
                          />
                        </div>
                        <div className="flex items-center justify-between text-xs text-slate-400 mt-1">
                          <span className="truncate max-w-[200px]">
                            {jobProgress.currentFile ||
                              (jobProgress.filesScanned
                                ? `已扫描 ${jobProgress.filesScanned} 个文件`
                                : "")}
                          </span>
                          <div className="flex items-center gap-2 flex-shrink-0">
                            {/* 已传输字节数（带动画） */}
                            {jobProgress.bytesTotal > 0 && (
                              <AnimatedBytes 
                                transferred={jobProgress.bytesTransferred} 
                                total={jobProgress.bytesTotal} 
                              />
                            )}
                            {/* 速度（带动画） */}
                            <AnimatedSpeed speed={jobProgress.speed} />
                          </div>
                        </div>
                      </div>
                    )}

                    {/* 完成状态 */}
                    {jobProgress?.status === "completed" && (
                      <div className="mt-2 pt-2 border-t border-slate-100 dark:border-slate-800">
                        <div className="flex items-center gap-1.5 text-green-600 dark:text-green-500 text-xs">
                          <RefreshCw className="w-3 h-3" />
                          <span>
                            完成，共 {jobProgress.filesCompleted} 个文件
                            {jobProgress.startTime > 0 && jobProgress.endTime > 0 && (() => {
                              const duration = jobProgress.endTime - jobProgress.startTime;
                              if (duration < 60) return `，用时 ${duration}秒`;
                              if (duration < 3600) return `，用时 ${Math.floor(duration / 60)}分${duration % 60}秒`;
                              return `，用时 ${Math.floor(duration / 3600)}时${Math.floor((duration % 3600) / 60)}分`;
                            })()}
                          </span>
                        </div>
                      </div>
                    )}

                    {/* 失败状态 */}
                    {jobProgress?.status === "failed" && (
                      <div className="mt-2 pt-2 border-t border-slate-100 dark:border-slate-800">
                        <div className="text-red-500 text-xs">
                          同步失败，请检查配置后重试
                        </div>
                      </div>
                    )}
                  </div>
                </div>
              );
            });
          })()}
        </div>
      </main>

      {/* 创建/编辑任务对话框 */}
      <CreateJobDialog
        isOpen={isDialogOpen || editingJob !== null}
        onClose={() => {
          setIsDialogOpen(false);
          setEditingJob(null);
        }}
        onJobCreated={loadJobs}
        editJob={editingJob}
      />

      {/* 设置对话框 */}
      <SettingsDialog
        isOpen={isSettingsOpen}
        onClose={() => setIsSettingsOpen(false)}
      />

      {/* 历史记录面板 */}
      <HistoryPanel
        isOpen={historyJob !== null}
        onClose={() => setHistoryJob(null)}
        jobId={historyJob?.id || ""}
        jobName={historyJob?.name || ""}
      />

      {/* 删除确认对话框 */}
      <ConfirmDialog
        isOpen={deletingJob !== null}
        title="删除任务"
        message={`确定要删除任务 "${deletingJob?.name}" 吗？此操作不可撤销。`}
        confirmText="删除"
        cancelText="取消"
        danger
        onConfirm={() => deletingJob && handleDeleteJob(deletingJob.id)}
        onCancel={() => setDeletingJob(null)}
      />

      {/* 差异视图对话框 */}
      <DiffViewDialog
        isOpen={diffResult !== null}
        onClose={() => {
          setDiffResult(null);
          setDiffJobId(null);
        }}
        diffResult={diffResult}
        onSync={handleSyncFromDiff}
      />

      {/* 冲突处理对话框 */}
      <ConflictDialog
        isOpen={conflicts.length > 0}
        onClose={() => {
          setConflicts([]);
          setConflictJobId(null);
        }}
        conflicts={conflicts}
        onResolve={handleConflictResolve}
      />

      {/* Toast 通知 */}
      <ToastContainer toasts={toasts} onClose={closeToast} />
    </div>
  );
}

export default App;
