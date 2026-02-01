import { useState, useEffect, useCallback } from "react";

/**
 * 弹窗状态管理 Hook
 * 统一处理所有弹窗的打开/关闭动画逻辑
 */
export function useDialog(isOpen: boolean, onClose: () => void) {
  const [visible, setVisible] = useState(false);
  const [isClosing, setIsClosing] = useState(false);

  useEffect(() => {
    if (isOpen) {
      setVisible(true);
      setIsClosing(false);
    }
  }, [isOpen]);

  const handleClose = useCallback(
    (callback?: () => void) => {
      setIsClosing(true);
      setTimeout(() => {
        setVisible(false);
        if (callback) {
          callback();
        } else {
          onClose();
        }
      }, 100);
    },
    [onClose]
  );

  return { visible, isClosing, handleClose };
}
