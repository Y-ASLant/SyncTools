import { X } from "lucide-react";
import { useDialog } from "../hooks";

type MaxWidth = "sm" | "md" | "lg" | "xl" | "2xl" | "3xl" | "4xl";

interface BaseDialogProps {
  isOpen: boolean;
  onClose: () => void;
  title?: string;
  maxWidth?: MaxWidth;
  showHeader?: boolean;
  showCloseButton?: boolean;
  children: React.ReactNode;
  header?: React.ReactNode;
  footer?: React.ReactNode;
  zIndex?: number;
  className?: string;
}

const maxWidthClasses: Record<MaxWidth, string> = {
  sm: "max-w-sm",
  md: "max-w-md",
  lg: "max-w-lg",
  xl: "max-w-xl",
  "2xl": "max-w-2xl",
  "3xl": "max-w-3xl",
  "4xl": "max-w-4xl",
};

/**
 * 基础弹窗组件
 * 统一处理弹窗的容器、动画、头部和底部
 */
export function BaseDialog({
  isOpen,
  onClose,
  title,
  maxWidth = "md",
  showHeader = true,
  showCloseButton = true,
  children,
  header,
  footer,
  zIndex = 50,
  className = "",
}: BaseDialogProps) {
  const { visible, isClosing, handleClose } = useDialog(isOpen, onClose);

  if (!visible) return null;

  const hasHeader = showHeader && (title || header || showCloseButton);

  return (
    <div
      className={`fixed inset-0 flex items-center justify-center p-4 bg-black/50 ${
        isClosing ? "dialog-overlay-out" : "dialog-overlay"
      }`}
      style={{ zIndex }}
    >
      <div
        className={`w-full ${maxWidthClasses[maxWidth]} bg-white dark:bg-slate-800 rounded shadow-xl ${
          isClosing ? "dialog-content-out" : "dialog-content"
        } ${className}`}
      >
        {hasHeader && (
          <div className="flex items-center justify-between px-4 py-3 border-b border-slate-200 dark:border-slate-700">
            {header || (
              <h2 className="text-sm font-medium text-slate-900 dark:text-white">
                {title}
              </h2>
            )}
            {showCloseButton && (
              <button
                onClick={() => handleClose()}
                className="p-1 rounded hover:bg-slate-100 dark:hover:bg-slate-700 transition-colors"
              >
                <X className="w-4 h-4 text-slate-500" />
              </button>
            )}
          </div>
        )}
        {children}
        {footer && (
          <div className="flex items-center justify-end gap-2 px-4 py-3 border-t border-slate-200 dark:border-slate-700">
            {footer}
          </div>
        )}
      </div>
    </div>
  );
}

/**
 * 弹窗内容区域组件
 */
export function DialogContent({
  children,
  className = "",
}: {
  children: React.ReactNode;
  className?: string;
}) {
  return <div className={`px-4 py-3 ${className}`}>{children}</div>;
}

/**
 * 弹窗底部按钮组件
 */
export function DialogFooter({
  children,
  className = "",
}: {
  children: React.ReactNode;
  className?: string;
}) {
  return (
    <div
      className={`flex items-center justify-end gap-2 px-4 py-3 border-t border-slate-200 dark:border-slate-700 ${className}`}
    >
      {children}
    </div>
  );
}
