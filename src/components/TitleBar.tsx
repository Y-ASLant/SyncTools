import { useState, useEffect } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  Minus,
  X,
  Maximize2,
  Minimize2,
  Settings,
} from "lucide-react";

interface TitleBarProps {
  title?: string;
  onOpenSettings?: () => void;
}

export function TitleBar({
  title = "SyncTools",
  onOpenSettings,
}: TitleBarProps) {
  const [isMaximized, setIsMaximized] = useState(false);
  const [closeHover, setCloseHover] = useState(false);
  const appWindow = getCurrentWindow();

  // 窗口恢复显示时重置 hover 状态
  useEffect(() => {
    const unlisten = appWindow.onFocusChanged(({ payload: focused }) => {
      if (focused) setCloseHover(false);
    });
    return () => { unlisten.then(fn => fn()); };
  }, [appWindow]);

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

  const handleClose = () => {
    setCloseHover(false); // 先重置状态再关闭
    appWindow.close();
  };

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
          onMouseEnter={() => setCloseHover(true)}
          onMouseLeave={() => setCloseHover(false)}
          className={`w-7 h-7 flex items-center justify-center rounded transition-colors ${
            closeHover ? "bg-red-500" : ""
          }`}
          title="关闭"
        >
          <X className={`w-4 h-4 ${closeHover ? "text-white" : "text-slate-600 dark:text-slate-400"}`} />
        </button>
      </div>
    </div>
  );
}
