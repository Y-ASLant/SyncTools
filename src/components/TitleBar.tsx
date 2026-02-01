import { useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  Minus,
  X,
  Maximize2,
  Minimize2,
  Sun,
  Moon,
  Settings,
} from "lucide-react";

interface TitleBarProps {
  title?: string;
  isDarkMode?: boolean;
  onToggleDarkMode?: () => void;
  onOpenSettings?: () => void;
}

export function TitleBar({
  title = "SyncTools",
  isDarkMode,
  onToggleDarkMode,
  onOpenSettings,
}: TitleBarProps) {
  const [isMaximized, setIsMaximized] = useState(false);
  const appWindow = getCurrentWindow();

  const handleMinimize = () => appWindow.minimize();

  const handleMaximize = async () => {
    const maximized = await appWindow.isMaximized();
    if (maximized) {
      await appWindow.unmaximize();
      setIsMaximized(false);
    } else {
      await appWindow.maximize();
      setIsMaximized(true);
    }
  };

  const handleClose = () => appWindow.close();

  return (
    <div
      data-tauri-drag-region
      className="h-9 flex items-center justify-between bg-slate-100 dark:bg-slate-900 border-b border-slate-200 dark:border-slate-800 select-none"
    >
      {/* 左侧 Logo 和标题 */}
      <div
        data-tauri-drag-region
        className="flex items-center gap-2 px-3 h-full"
      >
        <span className="relative flex h-2 w-2">
          <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-blue-400 opacity-75"></span>
          <span className="relative inline-flex rounded-full h-2 w-2 bg-blue-500"></span>
        </span>
        <span className="text-sm font-medium text-slate-700 dark:text-slate-300">
          {title}
        </span>
      </div>

      {/* 右侧按钮区 */}
      <div className="flex h-full items-center gap-0.5 px-1">
        {/* 功能按钮 */}
        {onToggleDarkMode && (
          <button
            onClick={onToggleDarkMode}
            className="w-7 h-7 flex items-center justify-center rounded hover:bg-slate-200 dark:hover:bg-slate-700 transition-colors"
            title="切换主题"
          >
            {isDarkMode ? (
              <Sun className="w-4 h-4 text-slate-500 dark:text-slate-400" />
            ) : (
              <Moon className="w-4 h-4 text-slate-500 dark:text-slate-400" />
            )}
          </button>
        )}
        {onOpenSettings && (
          <button
            onClick={onOpenSettings}
            className="w-7 h-7 flex items-center justify-center rounded hover:bg-slate-200 dark:hover:bg-slate-700 transition-colors"
            title="设置"
          >
            <Settings className="w-4 h-4 text-slate-500 dark:text-slate-400" />
          </button>
        )}

        {/* 分隔线 */}
        <div className="w-px h-4 bg-slate-300 dark:bg-slate-700 mx-1" />

        {/* 窗口控制按钮 */}
        <button
          onClick={handleMinimize}
          className="w-7 h-7 flex items-center justify-center rounded hover:bg-slate-200 dark:hover:bg-slate-700 transition-colors"
          title="最小化"
        >
          <Minus className="w-4 h-4 text-slate-600 dark:text-slate-400" />
        </button>
        <button
          onClick={handleMaximize}
          className="w-7 h-7 flex items-center justify-center rounded hover:bg-slate-200 dark:hover:bg-slate-700 transition-colors"
          title={isMaximized ? "还原" : "最大化"}
        >
          {isMaximized ? (
            <Minimize2 className="w-3.5 h-3.5 text-slate-600 dark:text-slate-400" />
          ) : (
            <Maximize2 className="w-3.5 h-3.5 text-slate-600 dark:text-slate-400" />
          )}
        </button>
        <button
          onClick={handleClose}
          className="w-7 h-7 flex items-center justify-center rounded hover:bg-red-500 transition-colors group"
          title="关闭"
        >
          <X className="w-4 h-4 text-slate-600 dark:text-slate-400 group-hover:text-white" />
        </button>
      </div>
    </div>
  );
}
