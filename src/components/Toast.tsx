import { useEffect, useState, useCallback } from "react";
import { CheckCircle, XCircle, AlertCircle, Info, X } from "lucide-react";
import { cn } from "../lib/utils";
import {
  TOAST_DEFAULT_DURATION,
  TOAST_ERROR_DURATION,
  TOAST_EXIT_ANIMATION_DELAY,
  Z_INDEX_TOAST,
} from "../lib/constants";

export type ToastType = "success" | "error" | "warning" | "info";

export interface ToastMessage {
  id: string;
  type: ToastType;
  title: string;
  message?: string;
  duration?: number;
}

interface ToastProps {
  toast: ToastMessage;
  onClose: (id: string) => void;
}

const icons: Record<ToastType, React.ReactNode> = {
  success: <CheckCircle className="w-4 h-4 text-green-500" />,
  error: <XCircle className="w-4 h-4 text-red-500" />,
  warning: <AlertCircle className="w-4 h-4 text-yellow-500" />,
  info: <Info className="w-4 h-4 text-blue-500" />,
};

const bgColors: Record<ToastType, string> = {
  success:
    "bg-green-50 dark:bg-green-900/20 border-green-200 dark:border-green-800",
  error: "bg-red-50 dark:bg-red-900/20 border-red-200 dark:border-red-800",
  warning:
    "bg-yellow-50 dark:bg-yellow-900/20 border-yellow-200 dark:border-yellow-800",
  info: "bg-blue-50 dark:bg-blue-900/20 border-blue-200 dark:border-blue-800",
};

function Toast({ toast, onClose }: ToastProps) {
  const [isExiting, setIsExiting] = useState(false);

  const handleClose = useCallback(() => {
    setIsExiting(true);
    setTimeout(() => onClose(toast.id), TOAST_EXIT_ANIMATION_DELAY);
  }, [onClose, toast.id]);

  useEffect(() => {
    if (toast.duration !== 0) {
      const timer = setTimeout(() => {
        handleClose();
      }, toast.duration || TOAST_DEFAULT_DURATION);
      return () => clearTimeout(timer);
    }
  }, [toast.id, toast.duration, handleClose]);

  return (
    <div
      className={cn(
        "flex items-start gap-2 p-3 rounded border shadow-lg",
        isExiting ? "animate-slide-out" : "animate-slide-in",
        bgColors[toast.type],
      )}
    >
      {icons[toast.type]}
      <div className="flex-1 min-w-0">
        <p className="text-sm font-medium text-slate-900 dark:text-white">
          {toast.title}
        </p>
        {toast.message && (
          <p className="mt-0.5 text-xs text-slate-600 dark:text-slate-400 break-words">
            {toast.message}
          </p>
        )}
      </div>
      <button
        onClick={handleClose}
        className="p-0.5 rounded hover:bg-black/5 dark:hover:bg-white/5 transition-colors"
      >
        <X className="w-3.5 h-3.5 text-slate-500" />
      </button>
    </div>
  );
}

interface ToastContainerProps {
  toasts: ToastMessage[];
  onClose: (id: string) => void;
}

export function ToastContainer({ toasts, onClose }: ToastContainerProps) {
  if (toasts.length === 0) return null;

  return (
    <div className={`fixed bottom-4 right-4 z-${Z_INDEX_TOAST} flex flex-col gap-2 max-w-sm`}>
      {toasts.map((toast) => (
        <Toast key={toast.id} toast={toast} onClose={onClose} />
      ))}
    </div>
  );
}

// Toast hook
let toastId = 0;
let toastListeners: ((toasts: ToastMessage[]) => void)[] = [];
let currentToasts: ToastMessage[] = [];

function notifyListeners() {
  toastListeners.forEach((listener) => listener([...currentToasts]));
}

export function useToast() {
  const [toasts, setToasts] = useState<ToastMessage[]>(currentToasts);

  useEffect(() => {
    const listener = (newToasts: ToastMessage[]) => setToasts(newToasts);
    toastListeners.push(listener);
    return () => {
      toastListeners = toastListeners.filter((l) => l !== listener);
    };
  }, []);

  const showToast = (
    type: ToastType,
    title: string,
    message?: string,
    duration?: number,
  ) => {
    const id = `toast-${++toastId}`;
    const toast: ToastMessage = { id, type, title, message, duration };
    currentToasts = [...currentToasts, toast];
    notifyListeners();
    return id;
  };

  const closeToast = (id: string) => {
    currentToasts = currentToasts.filter((t) => t.id !== id);
    notifyListeners();
  };

  return {
    toasts,
    showToast,
    closeToast,
    success: (title: string, message?: string) =>
      showToast("success", title, message),
    error: (title: string, message?: string) =>
      showToast("error", title, message, TOAST_ERROR_DURATION),
    warning: (title: string, message?: string) =>
      showToast("warning", title, message),
    info: (title: string, message?: string) =>
      showToast("info", title, message),
  };
}
