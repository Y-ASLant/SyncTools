import { AlertTriangle } from "lucide-react";
import { useDialog } from "../hooks";

interface ConfirmDialogProps {
  isOpen: boolean;
  title: string;
  message: string;
  confirmText?: string;
  cancelText?: string;
  danger?: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}

export function ConfirmDialog({
  isOpen,
  title,
  message,
  confirmText = "确定",
  cancelText = "取消",
  danger = false,
  onConfirm,
  onCancel,
}: ConfirmDialogProps) {
  const { visible, isClosing, handleClose } = useDialog(isOpen, onCancel);

  if (!visible) return null;

  return (
    <div
      className={`fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/50 ${
        isClosing ? "dialog-overlay-out" : "dialog-overlay"
      }`}
    >
      <div
        className={`w-full max-w-sm bg-white dark:bg-slate-800 rounded shadow-xl ${
          isClosing ? "dialog-content-out" : "dialog-content"
        }`}
      >
        <div className="p-4">
          <div className="flex items-start gap-3">
            {danger && (
              <div className="flex-shrink-0 w-8 h-8 rounded bg-red-100 dark:bg-red-900/30 flex items-center justify-center">
                <AlertTriangle className="w-4 h-4 text-red-500" />
              </div>
            )}
            <div className="flex-1">
              <h3 className="text-sm font-medium text-slate-900 dark:text-white">
                {title}
              </h3>
              <p className="mt-1 text-xs text-slate-500 dark:text-slate-400">
                {message}
              </p>
            </div>
          </div>
        </div>
        <div className="flex items-center justify-end gap-2 px-4 py-3 border-t border-slate-200 dark:border-slate-700">
          <button
            onClick={() => handleClose(onCancel)}
            className="px-3 py-1.5 rounded text-xs text-slate-700 dark:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-700 transition-colors btn-press"
          >
            {cancelText}
          </button>
          <button
            onClick={() => handleClose(onConfirm)}
            className={`px-3 py-1.5 rounded text-xs text-white transition-colors btn-press ${
              danger
                ? "bg-red-500 hover:bg-red-600"
                : "bg-blue-500 hover:bg-blue-600"
            }`}
          >
            {confirmText}
          </button>
        </div>
      </div>
    </div>
  );
}
