import { create } from "zustand";
import { persist } from "zustand/middleware";
import type { SyncJob, SyncProgress } from "./types";

interface SyncStore {
  // 状态
  jobs: SyncJob[];
  progress: Record<string, SyncProgress>;
  isDarkMode: boolean;

  // Actions
  addJob: (job: SyncJob) => void;
  updateJob: (id: string, updates: Partial<SyncJob>) => void;
  removeJob: (id: string) => void;
  getJob: (id: string) => SyncJob | undefined;

  setProgress: (jobId: string, progress: SyncProgress) => void;
  clearProgress: (jobId: string) => void;

  toggleDarkMode: () => void;
  setDarkMode: (isDark: boolean) => void;
}

export const useSyncStore = create<SyncStore>()(
  persist(
    (set, get) => ({
      // 初始状态
      jobs: [],
      progress: {},
      isDarkMode: false,

      // 任务管理
      addJob: (job) => set((state) => ({ jobs: [...state.jobs, job] })),

      updateJob: (id, updates) =>
        set((state) => ({
          jobs: state.jobs.map((job) =>
            job.id === id ? { ...job, ...updates, updatedAt: Date.now() } : job,
          ),
        })),

      removeJob: (id) =>
        set((state) => ({
          jobs: state.jobs.filter((job) => job.id !== id),
        })),

      getJob: (id) => get().jobs.find((job) => job.id === id),

      // 进度管理
      setProgress: (jobId, progress) =>
        set((state) => ({
          progress: { ...state.progress, [jobId]: progress },
        })),

      clearProgress: (jobId) =>
        set((state) => {
          const newProgress = { ...state.progress };
          delete newProgress[jobId];
          return { progress: newProgress };
        }),

      clearAllProgress: () => set({ progress: {} }),

      // 主题切换
      toggleDarkMode: () => {
        const newMode = !get().isDarkMode;
        set({ isDarkMode: newMode });
        // 更新 DOM
        if (newMode) {
          document.documentElement.classList.add("dark");
        } else {
          document.documentElement.classList.remove("dark");
        }
        // 保存到 localStorage
        localStorage.setItem("dark-mode", String(newMode));
      },

      setDarkMode: (isDark: boolean) => {
        set({ isDarkMode: isDark });
        if (isDark) {
          document.documentElement.classList.add("dark");
        } else {
          document.documentElement.classList.remove("dark");
        }
        localStorage.setItem("dark-mode", String(isDark));
      },
    }),
    {
      name: "synctools-storage",
      partialize: (state) => ({
        jobs: state.jobs,
        isDarkMode: state.isDarkMode,
      }),
      onRehydrateStorage: () => (state) => {
        // 恢复时同步 DOM
        if (state?.isDarkMode) {
          document.documentElement.classList.add("dark");
        } else {
          document.documentElement.classList.remove("dark");
        }
      },
    },
  ),
);
