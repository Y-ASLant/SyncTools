import { StrictMode } from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import App from "./App";
import "./index.css";

// 初始化深色模式（index.html 中已提前处理，这里做兜底）
const initDarkMode = () => {
  const isDark = localStorage.getItem("dark-mode") === "true";
  if (isDark) {
    document.documentElement.classList.add("dark");
  } else {
    document.documentElement.classList.remove("dark");
  }
};

initDarkMode();

// 渲染应用
ReactDOM.createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);

// 应用渲染完成后显示窗口
// 使用 requestIdleCallback 确保 React 完成首次渲染
const showWindow = () => {
  getCurrentWindow().show();
};

if ("requestIdleCallback" in window) {
  requestIdleCallback(showWindow);
} else {
  // 降级方案
  setTimeout(showWindow, 100);
}
