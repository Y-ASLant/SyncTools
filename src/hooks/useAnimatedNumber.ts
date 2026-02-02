import { useState, useEffect, useRef } from "react";

/**
 * 平滑数字动画 Hook（追逐式插值）
 * 使用线性插值持续追逐目标值，避免跳跃和闪烁
 * @param targetValue 目标值
 * @param smoothing 平滑系数 0-1，越小越平滑（默认 0.15）
 */
export function useAnimatedNumber(targetValue: number, smoothing = 0.15): number {
  const [displayValue, setDisplayValue] = useState(targetValue);
  const currentRef = useRef(targetValue);
  const targetRef = useRef(targetValue);
  const animationRef = useRef<number | null>(null);

  // 更新目标值
  targetRef.current = targetValue;

  useEffect(() => {
    // 初始化时直接设置
    if (currentRef.current === 0 && targetValue > 0) {
      currentRef.current = targetValue;
      setDisplayValue(targetValue);
      return;
    }

    const animate = () => {
      const current = currentRef.current;
      const target = targetRef.current;
      const diff = target - current;

      // 如果差值很小，直接到达目标
      if (Math.abs(diff) < 1) {
        if (current !== target) {
          currentRef.current = target;
          setDisplayValue(target);
        }
      } else {
        // 线性插值追逐目标
        const newValue = current + diff * smoothing;
        currentRef.current = newValue;
        setDisplayValue(Math.round(newValue));
      }

      animationRef.current = requestAnimationFrame(animate);
    };

    animationRef.current = requestAnimationFrame(animate);

    return () => {
      if (animationRef.current) {
        cancelAnimationFrame(animationRef.current);
      }
    };
  }, []); // 只在挂载时启动动画循环

  return displayValue;
}
