import { useState, useEffect, useRef } from "react";
import {
  X,
  Monitor,
  Moon,
  Sun,
  FolderPlus,
  Zap,
  ChevronDown,
  Check,
  Database,
  FileText,
  HardDrive,
} from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { cn } from "../lib/utils";
import { useSyncStore } from "../lib/store";
import { MessageDialog } from "./MessageDialog";
import type { LogConfig } from "../lib/types";

// shadcn 风格的 Select 组件
interface SelectOption {
  value: number;
  label: string;
  description?: string;
}

interface SelectProps {
  value: number;
  onChange: (value: number) => void;
  options: SelectOption[];
}

function Select({ value, onChange, options }: SelectProps) {
  const [isOpen, setIsOpen] = useState(false);
  const selectRef = useRef<HTMLDivElement>(null);

  const selectedOption = options.find((opt) => opt.value === value);

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (
        selectRef.current &&
        !selectRef.current.contains(event.target as Node)
      ) {
        setIsOpen(false);
      }
    };
    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, []);

  return (
    <div ref={selectRef} className="relative w-[120px]">
      <button
        type="button"
        onClick={() => setIsOpen(!isOpen)}
        className={cn(
          "flex items-center justify-between gap-2 h-8 px-3 w-full text-xs rounded-md border bg-white dark:bg-slate-800 text-slate-900 dark:text-white shadow-sm transition-colors",
          isOpen
            ? "border-blue-500 ring-2 ring-blue-500/20"
            : "border-slate-200 dark:border-slate-600 hover:border-slate-300 dark:hover:border-slate-500",
        )}
      >
        <span>{selectedOption?.label}</span>
        <ChevronDown
          className={cn(
            "w-3.5 h-3.5 text-slate-500 transition-transform",
            isOpen && "rotate-180",
          )}
        />
      </button>

      {isOpen && (
        <div className="absolute right-0 top-full mt-1 z-50 w-full max-h-48 overflow-y-auto rounded-md border border-slate-200 dark:border-slate-600 bg-white dark:bg-slate-800 shadow-lg py-1 animate-in fade-in-0 zoom-in-95 scrollable">
          {options.map((option) => (
            <button
              key={option.value}
              type="button"
              onClick={() => {
                onChange(option.value);
                setIsOpen(false);
              }}
              className={cn(
                "flex items-center justify-between w-full px-3 py-1.5 text-xs text-left transition-colors",
                option.value === value
                  ? "bg-blue-50 dark:bg-blue-900/20 text-blue-600 dark:text-blue-400"
                  : "text-slate-700 dark:text-slate-300 hover:bg-slate-50 dark:hover:bg-slate-700/50",
              )}
            >
              <span>{option.label}</span>
              {option.value === value && <Check className="w-3.5 h-3.5" />}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

interface SettingsDialogProps {
  isOpen: boolean;
  onClose: () => void;
}

type Theme = "system" | "light" | "dark";

// 消息弹窗组件
export function SettingsDialog({ isOpen, onClose }: SettingsDialogProps) {
  const { setDarkMode } = useSyncStore();
  const [theme, setTheme] = useState<Theme>("system");
  const [autoCreateDir, setAutoCreateDir] = useState(true);
  const [maxConcurrent, setMaxConcurrent] = useState(4);
  const [dataPath, setDataPath] = useState("");
  const [visible, setVisible] = useState(false);
  const [isClosing, setIsClosing] = useState(false);
  const [isMigrating, setIsMigrating] = useState(false);
  const [messageDialog, setMessageDialog] = useState<{
    isOpen: boolean;
    title: string;
    message: string;
    type: "info" | "success" | "error";
  }>({ isOpen: false, title: "", message: "", type: "info" });

  // 日志配置状态
  const [logEnabled, setLogEnabled] = useState(true);
  const [logMaxSize, setLogMaxSize] = useState(5);

  useEffect(() => {
    if (isOpen) {
      setVisible(true);
      setIsClosing(false);
      // 加载数据路径
      invoke<string>("get_data_path").then(setDataPath).catch(console.error);
      // 加载日志配置
      invoke<LogConfig>("get_log_config")
        .then((config) => {
          setLogEnabled(config.enabled);
          setLogMaxSize(config.maxSizeMb);
        })
        .catch(console.error);
    }
  }, [isOpen]);

  const showMessage = (
    title: string,
    message: string,
    type: "info" | "success" | "error" = "info",
  ) => {
    setMessageDialog({ isOpen: true, title, message, type });
  };

  const handleLogConfigChange = async (enabled?: boolean, maxSizeMb?: number) => {
    try {
      const newEnabled = enabled ?? logEnabled;
      const newMaxSize = maxSizeMb ?? logMaxSize;
      
      await invoke("set_log_config", {
        enabled: newEnabled,
        maxSizeMb: newMaxSize,
      });
      
      if (enabled !== undefined) setLogEnabled(newEnabled);
      if (maxSizeMb !== undefined) setLogMaxSize(newMaxSize);
    } catch (err) {
      console.error("保存日志配置失败:", err);
    }
  };

  const handleChangeDataPath = async () => {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: "选择数据保存位置",
        defaultPath: dataPath,
      });
      if (selected && typeof selected === "string") {
        setIsMigrating(true);
        try {
          const result = await invoke<string>("set_data_path", {
            path: selected,
          });
          setDataPath(selected);
          showMessage("迁移成功", `${result}\n\n重启应用后生效。`, "success");
        } catch (err) {
          showMessage("迁移失败", String(err), "error");
        } finally {
          setIsMigrating(false);
        }
      }
    } catch (err) {
      console.error("修改数据目录失败:", err);
      showMessage("操作失败", String(err), "error");
    }
  };

  const handleClose = () => {
    setIsClosing(true);
    setTimeout(() => {
      setVisible(false);
      onClose();
    }, 100);
  };

  useEffect(() => {
    // 从 localStorage 读取主题设置
    const savedTheme = localStorage.getItem("theme-preference") as Theme;
    if (savedTheme) {
      setTheme(savedTheme);
    }
    // 读取自动创建目录设置
    const savedAutoCreate = localStorage.getItem("auto-create-dir");
    if (savedAutoCreate !== null) {
      setAutoCreateDir(savedAutoCreate === "true");
    }
    // 读取并行数设置
    const savedConcurrent = localStorage.getItem("max-concurrent");
    if (savedConcurrent !== null) {
      setMaxConcurrent(parseInt(savedConcurrent) || 4);
    }
  }, []);

  const handleThemeChange = (newTheme: Theme) => {
    setTheme(newTheme);
    localStorage.setItem("theme-preference", newTheme);

    if (newTheme === "dark") {
      setDarkMode(true);
    } else if (newTheme === "light") {
      setDarkMode(false);
    } else {
      // system
      const prefersDark = window.matchMedia(
        "(prefers-color-scheme: dark)",
      ).matches;
      setDarkMode(prefersDark);
    }
  };

  if (!visible) return null;

  return (
    <div
      className={`fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/50 ${isClosing ? "dialog-overlay-out" : "dialog-overlay"}`}
    >
      <div
        className={`w-full max-w-3xl bg-white dark:bg-slate-800 rounded shadow-xl ${isClosing ? "dialog-content-out" : "dialog-content"}`}
      >
        {/* 头部 */}
        <div className="flex items-center justify-between px-4 py-3 border-b border-slate-200 dark:border-slate-700">
          <h2 className="text-sm font-medium text-slate-900 dark:text-white">
            设置
          </h2>
          <button
            onClick={handleClose}
            className="p-1 rounded hover:bg-slate-100 dark:hover:bg-slate-700 transition-colors"
          >
            <X className="w-4 h-4 text-slate-500" />
          </button>
        </div>

        {/* 内容 - 四宫格布局 */}
        <div className="p-4 grid grid-cols-2 gap-4">
          {/* 左上 - 外观 */}
          <div className="flex flex-col">
            <h3 className="text-xs font-medium text-slate-500 dark:text-slate-400 mb-2 uppercase tracking-wider">
              外观
            </h3>
            <div className="flex-1 flex items-center">
            <div className="grid grid-cols-3 gap-2 w-full">
              <button
                onClick={() => handleThemeChange("light")}
                className={cn(
                  "flex items-center justify-center gap-1.5 px-2 py-3 rounded-lg border transition-all",
                  theme === "light"
                    ? "border-orange-500 bg-orange-50 dark:bg-orange-900/20 shadow-sm"
                    : "border-slate-200 dark:border-slate-700 hover:border-slate-300 dark:hover:border-slate-600 hover:bg-slate-50 dark:hover:bg-slate-700/50",
                )}
              >
                <Sun
                  className={cn(
                    "w-4 h-4",
                    theme === "light" ? "text-orange-500" : "text-slate-500",
                  )}
                />
                <span className="text-xs font-medium text-slate-900 dark:text-white">
                  浅色
                </span>
              </button>
              <button
                onClick={() => handleThemeChange("dark")}
                className={cn(
                  "flex items-center justify-center gap-1.5 px-2 py-3 rounded-lg border transition-all",
                  theme === "dark"
                    ? "border-indigo-500 bg-indigo-50 dark:bg-indigo-900/20 shadow-sm"
                    : "border-slate-200 dark:border-slate-700 hover:border-slate-300 dark:hover:border-slate-600 hover:bg-slate-50 dark:hover:bg-slate-700/50",
                )}
              >
                <Moon
                  className={cn(
                    "w-4 h-4",
                    theme === "dark" ? "text-indigo-500" : "text-slate-500",
                  )}
                />
                <span className="text-xs font-medium text-slate-900 dark:text-white">
                  深色
                </span>
              </button>
              <button
                onClick={() => handleThemeChange("system")}
                className={cn(
                  "flex items-center justify-center gap-1.5 px-2 py-3 rounded-lg border transition-all",
                  theme === "system"
                    ? "border-blue-500 bg-blue-50 dark:bg-blue-900/20 shadow-sm"
                    : "border-slate-200 dark:border-slate-700 hover:border-slate-300 dark:hover:border-slate-600 hover:bg-slate-50 dark:hover:bg-slate-700/50",
                )}
              >
                <Monitor
                  className={cn(
                    "w-4 h-4",
                    theme === "system" ? "text-blue-500" : "text-slate-500",
                  )}
                />
                <span className="text-xs font-medium text-slate-900 dark:text-white">
                  系统
                </span>
              </button>
            </div>
            </div>
          </div>

          {/* 右上 - 数据存储 */}
          <div>
            <h3 className="text-xs font-medium text-slate-500 dark:text-slate-400 mb-2 uppercase tracking-wider">
              数据存储
            </h3>
            <div className="flex items-center justify-between p-2 rounded-md hover:bg-slate-50 dark:hover:bg-slate-700/50 transition-colors">
              <div className="flex items-center gap-2 flex-1 min-w-0">
                <div className="w-7 h-7 rounded-md bg-purple-50 dark:bg-purple-900/20 flex items-center justify-center shrink-0">
                  <Database className="w-3.5 h-3.5 text-purple-500" />
                </div>
                <div className="min-w-0 flex-1">
                  <p className="text-xs font-medium text-slate-900 dark:text-white">
                    保存位置
                  </p>
                  <p
                    className="text-[10px] text-slate-500 dark:text-slate-400 truncate"
                    title={dataPath}
                  >
                    {dataPath || "加载中..."}
                  </p>
                </div>
              </div>
              <button
                onClick={handleChangeDataPath}
                disabled={isMigrating}
                className={cn(
                  "ml-2 px-2 py-1 rounded-md text-xs transition-colors shrink-0",
                  isMigrating
                    ? "text-slate-400 cursor-not-allowed"
                    : "text-blue-600 dark:text-blue-400 hover:bg-blue-50 dark:hover:bg-blue-900/20",
                )}
              >
                {isMigrating ? "迁移中..." : "修改"}
              </button>
            </div>
          </div>

          {/* 左下 - 同步设置 */}
          <div>
            <h3 className="text-xs font-medium text-slate-500 dark:text-slate-400 mb-2 uppercase tracking-wider">
              同步设置
            </h3>
            <div className="space-y-1">
              <div className="flex items-center justify-between p-2 rounded-md hover:bg-slate-50 dark:hover:bg-slate-700/50 transition-colors">
                <div className="flex items-center gap-2">
                  <div className="w-7 h-7 rounded-md bg-amber-50 dark:bg-amber-900/20 flex items-center justify-center">
                    <Zap className="w-3.5 h-3.5 text-amber-500" />
                  </div>
                  <p className="text-xs font-medium text-slate-900 dark:text-white">
                    并行传输数
                  </p>
                </div>
                <Select
                  value={maxConcurrent}
                  onChange={(value) => {
                    setMaxConcurrent(value);
                    localStorage.setItem("max-concurrent", String(value));
                  }}
                  options={[
                    { value: 1, label: "1" },
                    { value: 2, label: "2" },
                    { value: 4, label: "4 (推荐)" },
                    { value: 8, label: "8" },
                    { value: 16, label: "16" },
                    { value: 32, label: "32" },
                    { value: 64, label: "64" },
                    { value: 128, label: "128" },
                  ]}
                />
              </div>
              <label className="flex items-center justify-between p-2 rounded-md hover:bg-slate-50 dark:hover:bg-slate-700/50 transition-colors cursor-pointer">
                <div className="flex items-center gap-2">
                  <div className="w-7 h-7 rounded-md bg-green-50 dark:bg-green-900/20 flex items-center justify-center">
                    <FolderPlus className="w-3.5 h-3.5 text-green-500" />
                  </div>
                  <p className="text-xs font-medium text-slate-900 dark:text-white">
                    自动创建目录
                  </p>
                </div>
                <button
                  type="button"
                  role="switch"
                  aria-checked={autoCreateDir}
                  onClick={() => {
                    const newValue = !autoCreateDir;
                    setAutoCreateDir(newValue);
                    localStorage.setItem("auto-create-dir", String(newValue));
                  }}
                  className={cn(
                    "relative inline-flex h-5 w-9 shrink-0 cursor-pointer rounded-full transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-500 focus-visible:ring-offset-2 dark:focus-visible:ring-offset-slate-800",
                    autoCreateDir
                      ? "bg-blue-500"
                      : "bg-slate-200 dark:bg-slate-600",
                  )}
                >
                  <span
                    className={cn(
                      "pointer-events-none inline-block h-4 w-4 mt-0.5 ml-0.5 transform rounded-full bg-white shadow transition-transform",
                      autoCreateDir ? "translate-x-4" : "translate-x-0",
                    )}
                  />
                </button>
              </label>
            </div>
          </div>

          {/* 右下 - 日志 */}
          <div>
            <h3 className="text-xs font-medium text-slate-500 dark:text-slate-400 mb-2 uppercase tracking-wider">
              日志
            </h3>
            <div className="space-y-1">
              <label className="flex items-center justify-between p-2 rounded-md hover:bg-slate-50 dark:hover:bg-slate-700/50 transition-colors cursor-pointer">
                <div className="flex items-center gap-2">
                  <div className="w-7 h-7 rounded-md bg-blue-50 dark:bg-blue-900/20 flex items-center justify-center">
                    <FileText className="w-3.5 h-3.5 text-blue-500" />
                  </div>
                  <p className="text-xs font-medium text-slate-900 dark:text-white">
                    启用日志
                  </p>
                </div>
                <button
                  type="button"
                  role="switch"
                  aria-checked={logEnabled}
                  onClick={() => handleLogConfigChange(!logEnabled, undefined)}
                  className={cn(
                    "relative inline-flex h-5 w-9 shrink-0 cursor-pointer rounded-full transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-500 focus-visible:ring-offset-2 dark:focus-visible:ring-offset-slate-800",
                    logEnabled
                      ? "bg-blue-500"
                      : "bg-slate-200 dark:bg-slate-600",
                  )}
                >
                  <span
                    className={cn(
                      "pointer-events-none inline-block h-4 w-4 mt-0.5 ml-0.5 transform rounded-full bg-white shadow transition-transform",
                      logEnabled ? "translate-x-4" : "translate-x-0",
                    )}
                  />
                </button>
              </label>
              <div className="flex items-center justify-between p-2 rounded-md hover:bg-slate-50 dark:hover:bg-slate-700/50 transition-colors">
                <div className="flex items-center gap-2">
                  <div className="w-7 h-7 rounded-md bg-cyan-50 dark:bg-cyan-900/20 flex items-center justify-center">
                    <HardDrive className="w-3.5 h-3.5 text-cyan-500" />
                  </div>
                  <p className="text-xs font-medium text-slate-900 dark:text-white">
                    大小限制
                  </p>
                </div>
                <Select
                  value={logMaxSize}
                  onChange={(value) => handleLogConfigChange(undefined, value)}
                  options={[
                    { value: 1, label: "1 MB" },
                    { value: 5, label: "5 MB" },
                    { value: 10, label: "10 MB" },
                    { value: 20, label: "20 MB" },
                    { value: 50, label: "50 MB" },
                  ]}
                />
              </div>
            </div>
          </div>
        </div>
      </div>

      {/* 消息弹窗 */}
      <MessageDialog
        isOpen={messageDialog.isOpen}
        title={messageDialog.title}
        message={messageDialog.message}
        type={messageDialog.type}
        onClose={() => setMessageDialog({ ...messageDialog, isOpen: false })}
      />
    </div>
  );
}
