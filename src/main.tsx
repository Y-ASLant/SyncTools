import { StrictMode } from "react";
import ReactDOM from "react-dom/client";
import { emit } from "@tauri-apps/api/event";
import App from "./App";
import "./index.css";

// 初始化深色模式
const isDark = localStorage.getItem("dark-mode") === "true";
document.documentElement.classList.toggle("dark", isDark);

// 渲染应用
ReactDOM.createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);

// 渲染完成后通知后端显示窗口
requestAnimationFrame(() => {
  emit("frontend-ready");
});
