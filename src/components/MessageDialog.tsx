import { Check, X, AlertCircle } from "lucide-react";
import { useDialog } from "../hooks";

export interface MessageDialogProps {
  isOpen: boolean;
  title: string;
  message: string;
  type?: "info" | "success" | "error";
  onClose: () => void;
}

const iconColors = {
  info: "bg-blue-100 dark:bg-blue-900/30 text-blue-500",
  success: "bg-green-100 dark:bg-green-900/30 text-green-500",
  error: "bg-red-100 dark:bg-red-900/30 text-red-500",
};

export function MessageDialog({
  isOpen,
  title,
  message,
  type = "info",
  onClose,
}: MessageDialogProps) {
  const { visible, isClosing, handleClose } = useDialog(isOpen, onClose);

  if (!visible) return null;

  return (
    <div
      className={`fixed inset-0 z-[60] flex items-center justify-center p-4 bg-black/50 ${
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
            <div
              className={`flex-shrink-0 w-8 h-8 rounded-full flex items-center justify-center ${iconColors[type]}`}
            >
              {type === "success" ? (
                <Check className="w-4 h-4" />
              ) : type === "error" ? (
                <X className="w-4 h-4" />
              ) : (
                <AlertCircle className="w-4 h-4" />
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
            onClick={() => handleClose()}
            className="px-4 py-1.5 rounded text-xs bg-blue-500 hover:bg-blue-600 text-white transition-colors btn-press"
          >
            确定
          </button>
        </div>
      </div>
    </div>
  );
}
