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
} from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { cn } from "../lib/utils";
import { useSyncStore } from "../lib/store";

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
interface MessageDialogProps {
  isOpen: boolean;
  title: string;
  message: string;
  type?: "info" | "success" | "error";
  onClose: () => void;
}

function MessageDialog({
  isOpen,
  title,
  message,
  type = "info",
  onClose,
}: MessageDialogProps) {
  const [visible, setVisible] = useState(false);
  const [isClosing, setIsClosing] = useState(false);

  useEffect(() => {
    if (isOpen) {
      setVisible(true);
      setIsClosing(false);
    }
  }, [isOpen]);

  const handleClose = () => {
    setIsClosing(true);
    setTimeout(() => {
      setVisible(false);
      onClose();
    }, 100);
  };

  if (!visible) return null;

  const iconColors = {
    info: "bg-blue-100 dark:bg-blue-900/30 text-blue-500",
    success: "bg-green-100 dark:bg-green-900/30 text-green-500",
    error: "bg-red-100 dark:bg-red-900/30 text-red-500",
  };

  return (
    <div
      className={`fixed inset-0 z-[60] flex items-center justify-center p-4 bg-black/50 ${isClosing ? "dialog-overlay-out" : "dialog-overlay"}`}
    >
      <div
        className={`w-full max-w-sm bg-white dark:bg-slate-800 rounded shadow-xl ${isClosing ? "dialog-content-out" : "dialog-content"}`}
      >
        <div className="p-4">
          <div className="flex items-start gap-3">
            <div
              className={`flex-shrink-0 w-8 h-8 rounded-full flex items-center justify-center ${iconColors[type]}`}
            >
              {type === "success" ? (
                <Check className="w-4 h-4" />
              ) : type === "error" ? (
                <X className="w-4 h-4" />
              ) : (
                <Database className="w-4 h-4" />
              )}
            </div>
            <div className="flex-1">
              <h3 className="text-sm font-medium text-slate-900 dark:text-white">
                {title}
              </h3>
              <p className="mt-1 text-xs text-slate-500 dark:text-slate-400 whitespace-pre-line">
                {message}
              </p>
            </div>
          </div>
        </div>
        <div className="flex items-center justify-end px-4 py-3 border-t border-slate-200 dark:border-slate-700">
          <button
            onClick={handleClose}
            className="px-4 py-1.5 rounded text-xs bg-blue-500 hover:bg-blue-600 text-white transition-colors btn-press"
          >
            确定
          </button>
        </div>
      </div>
    </div>
  );
}

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

  useEffect(() => {
    if (isOpen) {
      setVisible(true);
      setIsClosing(false);
      // 加载数据路径
      invoke<string>("get_data_path").then(setDataPath).catch(console.error);
    }
  }, [isOpen]);

  const showMessage = (
    title: string,
    message: string,
    type: "info" | "success" | "error" = "info",
  ) => {
    setMessageDialog({ isOpen: true, title, message, type });
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
        className={`w-full max-w-md bg-white dark:bg-slate-800 rounded shadow-xl ${isClosing ? "dialog-content-out" : "dialog-content"}`}
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

        {/* 内容 */}
        <div className="p-4 space-y-5">
          {/* 外观设置 */}
          <div>
            <h3 className="text-xs font-medium text-slate-500 dark:text-slate-400 mb-3 uppercase tracking-wider">
              外观
            </h3>
            <div className="grid grid-cols-3 gap-2">
              <button
                onClick={() => handleThemeChange("light")}
                className={cn(
                  "flex flex-col items-center gap-2 p-3 rounded-lg border transition-all",
                  theme === "light"
                    ? "border-orange-500 bg-orange-50 dark:bg-orange-900/20 shadow-sm"
                    : "border-slate-200 dark:border-slate-700 hover:border-slate-300 dark:hover:border-slate-600 hover:bg-slate-50 dark:hover:bg-slate-700/50",
                )}
              >
                <div
                  className={cn(
                    "w-8 h-8 rounded-full flex items-center justify-center",
                    theme === "light"
                      ? "bg-orange-100 dark:bg-orange-900/30"
                      : "bg-slate-100 dark:bg-slate-700",
                  )}
                >
                  <Sun
                    className={cn(
                      "w-4 h-4",
                      theme === "light" ? "text-orange-500" : "text-slate-500",
                    )}
                  />
                </div>
                <span className="text-xs font-medium text-slate-900 dark:text-white">
                  浅色
                </span>
              </button>
              <button
                onClick={() => handleThemeChange("dark")}
                className={cn(
                  "flex flex-col items-center gap-2 p-3 rounded-lg border transition-all",
                  theme === "dark"
                    ? "border-indigo-500 bg-indigo-50 dark:bg-indigo-900/20 shadow-sm"
                    : "border-slate-200 dark:border-slate-700 hover:border-slate-300 dark:hover:border-slate-600 hover:bg-slate-50 dark:hover:bg-slate-700/50",
                )}
              >
                <div
                  className={cn(
                    "w-8 h-8 rounded-full flex items-center justify-center",
                    theme === "dark"
                      ? "bg-indigo-100 dark:bg-indigo-900/30"
                      : "bg-slate-100 dark:bg-slate-700",
                  )}
                >
                  <Moon
                    className={cn(
                      "w-4 h-4",
                      theme === "dark" ? "text-indigo-500" : "text-slate-500",
                    )}
                  />
                </div>
                <span className="text-xs font-medium text-slate-900 dark:text-white">
                  深色
                </span>
              </button>
              <button
                onClick={() => handleThemeChange("system")}
                className={cn(
                  "flex flex-col items-center gap-2 p-3 rounded-lg border transition-all",
                  theme === "system"
                    ? "border-blue-500 bg-blue-50 dark:bg-blue-900/20 shadow-sm"
                    : "border-slate-200 dark:border-slate-700 hover:border-slate-300 dark:hover:border-slate-600 hover:bg-slate-50 dark:hover:bg-slate-700/50",
                )}
              >
                <div
                  className={cn(
                    "w-8 h-8 rounded-full flex items-center justify-center",
                    theme === "system"
                      ? "bg-blue-100 dark:bg-blue-900/30"
                      : "bg-slate-100 dark:bg-slate-700",
                  )}
                >
                  <Monitor
                    className={cn(
                      "w-4 h-4",
                      theme === "system" ? "text-blue-500" : "text-slate-500",
                    )}
                  />
                </div>
                <span className="text-xs font-medium text-slate-900 dark:text-white">
                  系统
                </span>
              </button>
            </div>
          </div>

          {/* 同步设置 */}
          <div className="pt-4 border-t border-slate-200 dark:border-slate-700">
            <h3 className="text-xs font-medium text-slate-500 dark:text-slate-400 mb-3 uppercase tracking-wider">
              同步设置
            </h3>
            <div className="space-y-1">
              {/* 并行传输数 */}
              <div className="flex items-center justify-between p-2.5 rounded-md hover:bg-slate-50 dark:hover:bg-slate-700/50 transition-colors">
                <div className="flex items-center gap-2.5">
                  <div className="w-8 h-8 rounded-md bg-amber-50 dark:bg-amber-900/20 flex items-center justify-center">
                    <Zap className="w-4 h-4 text-amber-500" />
                  </div>
                  <div>
                    <p className="text-sm font-medium text-slate-900 dark:text-white">
                      并行传输数
                    </p>
                    <p className="text-xs text-slate-500 dark:text-slate-400">
                      同时传输的文件数量
                    </p>
                  </div>
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

              {/* 自动创建目录 */}
              <label className="flex items-center justify-between p-2.5 rounded-md hover:bg-slate-50 dark:hover:bg-slate-700/50 transition-colors cursor-pointer">
                <div className="flex items-center gap-2.5">
                  <div className="w-8 h-8 rounded-md bg-green-50 dark:bg-green-900/20 flex items-center justify-center">
                    <FolderPlus className="w-4 h-4 text-green-500" />
                  </div>
                  <div>
                    <p className="text-sm font-medium text-slate-900 dark:text-white">
                      自动创建目录
                    </p>
                    <p className="text-xs text-slate-500 dark:text-slate-400">
                      目标目录不存在时自动创建
                    </p>
                  </div>
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

          {/* 数据存储 */}
          <div className="pt-4 border-t border-slate-200 dark:border-slate-700">
            <h3 className="text-xs font-medium text-slate-500 dark:text-slate-400 mb-3 uppercase tracking-wider">
              数据存储
            </h3>
            <div className="flex items-center justify-between p-2.5 rounded-md hover:bg-slate-50 dark:hover:bg-slate-700/50 transition-colors">
              <div className="flex items-center gap-2.5 flex-1 min-w-0">
                <div className="w-8 h-8 rounded-md bg-purple-50 dark:bg-purple-900/20 flex items-center justify-center shrink-0">
                  <Database className="w-4 h-4 text-purple-500" />
                </div>
                <div className="min-w-0 flex-1">
                  <p className="text-sm font-medium text-slate-900 dark:text-white">
                    数据保存位置
                  </p>
                  <p
                    className="text-xs text-slate-500 dark:text-slate-400 truncate"
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
                  "ml-2 px-2.5 py-1 rounded-md text-xs transition-colors shrink-0",
                  isMigrating
                    ? "text-slate-400 cursor-not-allowed"
                    : "text-blue-600 dark:text-blue-400 hover:bg-blue-50 dark:hover:bg-blue-900/20",
                )}
              >
                {isMigrating ? "迁移中..." : "修改"}
              </button>
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
