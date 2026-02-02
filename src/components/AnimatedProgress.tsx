import { useAnimatedNumber } from "../hooks";

// 固定宽度的字节格式化（始终2位小数）
function formatBytesFixed(bytes: number): string {
  if (bytes === 0) return "0.00 B";
  const k = 1024;
  const sizes = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  const value = bytes / Math.pow(k, i);
  // 始终保持2位小数，确保宽度稳定
  return `${value.toFixed(2)} ${sizes[i]}`;
}

interface AnimatedBytesProps {
  transferred: number;
  total: number;
}

/**
 * 带平滑动画的字节进度显示
 */
export function AnimatedBytes({ transferred, total }: AnimatedBytesProps) {
  const animatedTransferred = useAnimatedNumber(transferred, 0.12);
  
  return (
    <span className="text-slate-500 font-mono text-[11px] inline-block text-right min-w-[150px]">
      {formatBytesFixed(animatedTransferred)}/{formatBytesFixed(total)}
    </span>
  );
}

interface AnimatedSpeedProps {
  speed: number;
}

/**
 * 带平滑动画的速度显示
 */
export function AnimatedSpeed({ speed }: AnimatedSpeedProps) {
  const animatedSpeed = useAnimatedNumber(speed, 0.08);
  
  if (animatedSpeed <= 0) return null;
  
  return (
    <span className="text-blue-500 font-mono text-[11px] inline-block text-right min-w-[85px]">
      {formatBytesFixed(animatedSpeed)}/s
    </span>
  );
}
