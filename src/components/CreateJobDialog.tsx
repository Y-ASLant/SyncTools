import React, { useState, useEffect, useCallback } from "react";
import {
  X,
  Folder,
  Cloud,
  Server,
  Check,
  Loader2,
  FolderOpen,
} from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { cn, getSyncModeLabel, getStorageTypeLabel } from "../lib/utils";
import { useDialog } from "../hooks";
import { MessageDialog } from "./MessageDialog";
import { DEFAULT_S3_REGION } from "../lib/constants";
import type {
  StorageType,
  SyncMode,
  TestConnectionResult,
  SyncJob,
} from "../lib/types";

interface CreateJobDialogProps {
  isOpen: boolean;
  onClose: () => void;
  onJobCreated: () => void;
  editJob?: SyncJob | null;
}

interface FormData {
  name: string;
  sourceType: StorageType;
  destType: StorageType;
  syncMode: SyncMode;
  // 源存储配置
  sourceLocalPath: string;
  sourceS3Bucket: string;
  sourceS3Region: string;
  sourceS3AccessKey: string;
  sourceS3SecretKey: string;
  sourceS3Endpoint: string;
  sourceWebdavEndpoint: string;
  sourceWebdavUsername: string;
  sourceWebdavPassword: string;
  // 目标存储配置
  destLocalPath: string;
  destS3Bucket: string;
  destS3Region: string;
  destS3AccessKey: string;
  destS3SecretKey: string;
  destS3Endpoint: string;
  destWebdavEndpoint: string;
  destWebdavUsername: string;
  destWebdavPassword: string;
}

const STORAGE_ICONS: Record<StorageType, React.ReactNode> = {
  local: <Folder className="w-4 h-4" />,
  s3: <Cloud className="w-4 h-4" />,
  webdav: <Server className="w-4 h-4" />,
};

export function CreateJobDialog({
  isOpen,
  onClose,
  onJobCreated,
  editJob,
}: CreateJobDialogProps) {
  const [step, setStep] = useState<1 | 2 | 3>(1);
  const [formData, setFormData] = useState<FormData>({
    name: "",
    sourceType: "local",
    destType: "s3",
    syncMode: "backup",
    // 源存储配置
    sourceLocalPath: "",
    sourceS3Bucket: "",
    sourceS3Region: DEFAULT_S3_REGION,
    sourceS3AccessKey: "",
    sourceS3SecretKey: "",
    sourceS3Endpoint: "",
    sourceWebdavEndpoint: "",
    sourceWebdavUsername: "",
    sourceWebdavPassword: "",
    // 目标存储配置
    destLocalPath: "",
    destS3Bucket: "",
    destS3Region: DEFAULT_S3_REGION,
    destS3AccessKey: "",
    destS3SecretKey: "",
    destS3Endpoint: "",
    destWebdavEndpoint: "",
    destWebdavUsername: "",
    destWebdavPassword: "",
  });

  const [isCreating, setIsCreating] = useState(false);
  const [testing, setTesting] = useState<Record<string, boolean>>({});
  const [testResults, setTestResults] = useState<
    Record<string, TestConnectionResult>
  >({});
  const [messageDialog, setMessageDialog] = useState<{
    isOpen: boolean;
    title: string;
    message: string;
    type: "info" | "success" | "error";
  }>({
    isOpen: false,
    title: "",
    message: "",
    type: "info",
  });

  const isEditing = !!editJob;

  const showMessage = (
    title: string,
    message: string,
    type: "info" | "success" | "error" = "info"
  ) => {
    setMessageDialog({ isOpen: true, title, message, type });
  };

  // 重置表单的回调
  const resetForm = useCallback(() => {
    setStep(1);
    setFormData({
      name: "",
      sourceType: "local",
      destType: "s3",
      syncMode: "backup",
      sourceLocalPath: "",
      sourceS3Bucket: "",
      sourceS3Region: DEFAULT_S3_REGION,
      sourceS3AccessKey: "",
      sourceS3SecretKey: "",
      sourceS3Endpoint: "",
      sourceWebdavEndpoint: "",
      sourceWebdavUsername: "",
      sourceWebdavPassword: "",
      destLocalPath: "",
      destS3Bucket: "",
      destS3Region: DEFAULT_S3_REGION,
      destS3AccessKey: "",
      destS3SecretKey: "",
      destS3Endpoint: "",
      destWebdavEndpoint: "",
      destWebdavUsername: "",
      destWebdavPassword: "",
    });
    setTestResults({});
    onClose();
  }, [onClose]);

  // 使用统一的弹窗 Hook
  const { visible, isClosing, handleClose } = useDialog(isOpen, resetForm);

  // 编辑模式下预填充数据
  useEffect(() => {
    if (editJob) {
      const sourceType = editJob.sourceConfig.type as StorageType;
      const destType = editJob.destConfig.type as StorageType;

      setFormData({
        name: editJob.name,
        sourceType,
        destType,
        syncMode: editJob.syncMode as SyncMode,
        // 源存储配置
        sourceLocalPath: editJob.sourceConfig.path || "",
        sourceS3Bucket: editJob.sourceConfig.bucket || "",
        sourceS3Region: editJob.sourceConfig.region || DEFAULT_S3_REGION,
        sourceS3AccessKey: editJob.sourceConfig.accessKey || "",
        sourceS3SecretKey: editJob.sourceConfig.secretKey || "",
        sourceS3Endpoint: editJob.sourceConfig.endpoint || "",
        sourceWebdavEndpoint: editJob.sourceConfig.webdavEndpoint || "",
        sourceWebdavUsername: editJob.sourceConfig.username || "",
        sourceWebdavPassword: editJob.sourceConfig.password || "",
        // 目标存储配置
        destLocalPath: editJob.destConfig.path || "",
        destS3Bucket: editJob.destConfig.bucket || "",
        destS3Region: editJob.destConfig.region || DEFAULT_S3_REGION,
        destS3AccessKey: editJob.destConfig.accessKey || "",
        destS3SecretKey: editJob.destConfig.secretKey || "",
        destS3Endpoint: editJob.destConfig.endpoint || "",
        destWebdavEndpoint: editJob.destConfig.webdavEndpoint || "",
        destWebdavUsername: editJob.destConfig.username || "",
        destWebdavPassword: editJob.destConfig.password || "",
      });
      setStep(3); // 编辑模式直接跳到配置页
    }
  }, [editJob]);

  if (!visible) return null;

  const testConnection = async (storage: "source" | "dest") => {
    const type = storage === "source" ? formData.sourceType : formData.destType;
    const key = `${storage}-${type}`;
    const isSource = storage === "source";

    setTesting((prev) => ({ ...prev, [key]: true }));
    setTestResults((prev) => ({
      ...prev,
      [key]: { success: false, message: "测试中...", details: null },
    }));

    try {
      const result = await invoke<TestConnectionResult>("test_connection", {
        typ: type,
        path: isSource ? formData.sourceLocalPath : formData.destLocalPath || null,
        bucket: isSource ? formData.sourceS3Bucket : formData.destS3Bucket || null,
        region: isSource ? formData.sourceS3Region : formData.destS3Region || null,
        accessKey: isSource ? formData.sourceS3AccessKey : formData.destS3AccessKey || null,
        secretKey: isSource ? formData.sourceS3SecretKey : formData.destS3SecretKey || null,
        endpoint: isSource ? formData.sourceS3Endpoint : formData.destS3Endpoint || null,
        webdavEndpoint: isSource ? formData.sourceWebdavEndpoint : formData.destWebdavEndpoint || null,
        username: isSource ? formData.sourceWebdavUsername : formData.destWebdavUsername || null,
        password: isSource ? formData.sourceWebdavPassword : formData.destWebdavPassword || null,
      });
      setTestResults((prev) => ({ ...prev, [key]: result }));
    } catch (error) {
      setTestResults((prev) => ({
        ...prev,
        [key]: { success: false, message: "测试失败", details: String(error) },
      }));
    } finally {
      setTesting((prev) => {
        const updated = { ...prev };
        delete updated[key];
        return updated;
      });
    }
  };

  // 验证表单
  const validateForm = (): string | null => {
    if (!formData.name.trim()) {
      return "请输入任务名称";
    }

    // 验证源存储配置
    if (formData.sourceType === "local") {
      if (!formData.sourceLocalPath.trim()) return "请输入源本地路径";
    } else if (formData.sourceType === "s3") {
      if (!formData.sourceS3Bucket.trim()) return "请输入源 S3 Bucket";
      if (!formData.sourceS3Region.trim()) return "请输入源 S3 Region";
      if (!formData.sourceS3AccessKey.trim()) return "请输入源 Access Key";
      if (!formData.sourceS3SecretKey.trim()) return "请输入源 Secret Key";
    } else if (formData.sourceType === "webdav") {
      if (!formData.sourceWebdavEndpoint.trim()) return "请输入源 WebDAV 地址";
      if (!formData.sourceWebdavUsername.trim()) return "请输入源 WebDAV 用户名";
      if (!formData.sourceWebdavPassword.trim()) return "请输入源 WebDAV 密码";
    }

    // 验证目标存储配置
    if (formData.destType === "local") {
      if (!formData.destLocalPath.trim()) return "请输入目标本地路径";
    } else if (formData.destType === "s3") {
      if (!formData.destS3Bucket.trim()) return "请输入目标 S3 Bucket";
      if (!formData.destS3Region.trim()) return "请输入目标 S3 Region";
      if (!formData.destS3AccessKey.trim()) return "请输入目标 Access Key";
      if (!formData.destS3SecretKey.trim()) return "请输入目标 Secret Key";
    } else if (formData.destType === "webdav") {
      if (!formData.destWebdavEndpoint.trim()) return "请输入目标 WebDAV 地址";
      if (!formData.destWebdavUsername.trim()) return "请输入目标 WebDAV 用户名";
      if (!formData.destWebdavPassword.trim()) return "请输入目标 WebDAV 密码";
    }

    return null;
  };

  const handleSubmit = async () => {
    // 验证表单
    const error = validateForm();
    if (error) {
      showMessage("提示", error, "info");
      return;
    }

    setIsCreating(true);
    try {
      const buildStorageConfig = (type: StorageType, isSource: boolean) => {
        switch (type) {
          case "local":
            return { type: "local", path: isSource ? formData.sourceLocalPath : formData.destLocalPath };
          case "s3":
            return {
              type: "s3",
              bucket: isSource ? formData.sourceS3Bucket : formData.destS3Bucket,
              region: isSource ? formData.sourceS3Region : formData.destS3Region,
              accessKey: isSource ? formData.sourceS3AccessKey : formData.destS3AccessKey,
              secretKey: isSource ? formData.sourceS3SecretKey : formData.destS3SecretKey,
              endpoint: (isSource ? formData.sourceS3Endpoint : formData.destS3Endpoint) || undefined,
            };
          case "webdav":
            return {
              type: "webdav",
              webdavEndpoint: isSource ? formData.sourceWebdavEndpoint : formData.destWebdavEndpoint,
              username: isSource ? formData.sourceWebdavUsername : formData.destWebdavUsername,
              password: isSource ? formData.sourceWebdavPassword : formData.destWebdavPassword,
            };
        }
      };

      if (isEditing && editJob) {
        // 编辑模式：更新任务
        await invoke("update_job", {
          id: editJob.id,
          name: formData.name,
          sourceConfig: buildStorageConfig(formData.sourceType, true),
          destConfig: buildStorageConfig(formData.destType, false),
          syncMode: formData.syncMode,
        });
      } else {
        // 创建模式：新建任务
        await invoke("create_job", {
          name: formData.name,
          sourceConfig: buildStorageConfig(formData.sourceType, true),
          destConfig: buildStorageConfig(formData.destType, false),
          syncMode: formData.syncMode,
          schedule: null,
        });
      }

      onJobCreated();
      handleClose();
    } catch (error) {
      console.error(isEditing ? "更新任务失败:" : "创建任务失败:", error);
      showMessage(
        isEditing ? "更新任务失败" : "创建任务失败",
        String(error),
        "error"
      );
    } finally {
      setIsCreating(false);
    }
  };

  const renderStep1 = () => (
    <div className="space-y-4">
      <div>
        <label className="block text-xs font-medium text-slate-700 dark:text-slate-300 mb-1.5">
          任务名称
        </label>
        <input
          type="text"
          value={formData.name}
          onChange={(e) => setFormData({ ...formData, name: e.target.value })}
          placeholder="例如: 备份文档到 S3"
          className="w-full px-3 py-1.5 rounded border border-slate-300 dark:border-slate-600 bg-white dark:bg-slate-800 text-sm text-slate-900 dark:text-white focus:ring-1 focus:ring-blue-500 focus:border-blue-500 outline-none transition-all"
        />
      </div>

      <div>
        <label className="block text-xs font-medium text-slate-700 dark:text-slate-300 mb-2">
          同步模式
        </label>
        <div className="grid grid-cols-3 gap-2">
          {(["bidirectional", "mirror", "backup"] as SyncMode[]).map((mode) => (
            <button
              key={mode}
              onClick={() => setFormData({ ...formData, syncMode: mode })}
              className={cn(
                "p-3 rounded border text-left transition-all",
                formData.syncMode === mode
                  ? "border-blue-500 bg-blue-50 dark:bg-blue-900/20"
                  : "border-slate-200 dark:border-slate-700 hover:border-slate-300 dark:hover:border-slate-600",
              )}
            >
              <div className="text-sm font-medium text-slate-900 dark:text-white mb-0.5">
                {getSyncModeLabel(mode)}同步
              </div>
              <div className="text-xs text-slate-500 dark:text-slate-400">
                {mode === "bidirectional"
                  ? "源 ↔ 目标，双向同步"
                  : mode === "mirror"
                    ? "源 → 目标，删除多余"
                    : "源 → 目标，仅新增"}
              </div>
            </button>
          ))}
        </div>
      </div>
    </div>
  );

  const renderStep2 = () => (
    <div className="space-y-4">
      <div>
        <label className="block text-xs font-medium text-slate-700 dark:text-slate-300 mb-2">
          源存储类型
        </label>
        <div className="grid grid-cols-3 gap-2">
          {(["local", "s3", "webdav"] as StorageType[]).map((type) => (
            <button
              key={type}
              onClick={() => setFormData({ ...formData, sourceType: type })}
              className={cn(
                "p-3 rounded border flex flex-col items-center gap-1.5 transition-all",
                formData.sourceType === type
                  ? "border-blue-500 bg-blue-50 dark:bg-blue-900/20"
                  : "border-slate-200 dark:border-slate-700 hover:border-slate-300 dark:hover:border-slate-600",
              )}
            >
              {STORAGE_ICONS[type]}
              <span className="text-sm font-medium text-slate-900 dark:text-white">
                {getStorageTypeLabel(type)}
              </span>
            </button>
          ))}
        </div>
      </div>

      <div>
        <label className="block text-xs font-medium text-slate-700 dark:text-slate-300 mb-2">
          目标存储类型
        </label>
        <div className="grid grid-cols-3 gap-2">
          {(["local", "s3", "webdav"] as StorageType[]).map((type) => (
            <button
              key={type}
              onClick={() => setFormData({ ...formData, destType: type })}
              className={cn(
                "p-3 rounded border flex flex-col items-center gap-1.5 transition-all",
                formData.destType === type
                  ? "border-green-500 bg-green-50 dark:bg-green-900/20"
                  : "border-slate-200 dark:border-slate-700 hover:border-slate-300 dark:hover:border-slate-600",
              )}
            >
              {STORAGE_ICONS[type]}
              <span className="text-sm font-medium text-slate-900 dark:text-white">
                {getStorageTypeLabel(type)}
              </span>
            </button>
          ))}
        </div>
      </div>
    </div>
  );

  const renderTestButton = (
    _label: string,
    storage: "source" | "dest",
    type: StorageType,
  ) => {
    if (type === "local") return null;

    const key = `${storage}-${type}`;
    const isLoading = testing[key];
    const result = testResults[key];

    return (
      <button
        onClick={() => testConnection(storage)}
        disabled={isLoading}
        className="mt-2 flex items-center gap-1.5 px-2 py-1 text-xs rounded border transition-colors"
        style={{
          borderColor: result
            ? result.success
              ? "#10b981"
              : "#ef4444"
            : "#e2e8f0",
        }}
      >
        {isLoading ? (
          <>
            <Loader2 className="w-3 h-3 animate-spin" />
            测试中...
          </>
        ) : result ? (
          <>
            {result.success ? (
              <Check className="w-3 h-3" />
            ) : (
              <X className="w-3 h-3" />
            )}
            {result.message}
          </>
        ) : (
          <>测试连接</>
        )}
      </button>
    );
  };

  const renderStep3 = () => (
    <div className="space-y-4">
      {formData.sourceType !== "local" && (
        <div className="p-3 border border-slate-200 dark:border-slate-700 rounded">
          <h4 className="text-xs font-medium text-slate-700 dark:text-slate-300 mb-2">
            源存储配置 ({formData.sourceType.toUpperCase()})
          </h4>
          {renderStorageConfig("source", formData.sourceType)}
          {renderTestButton("源", "source", formData.sourceType)}
        </div>
      )}

      {formData.destType !== "local" && (
        <div className="p-3 border border-slate-200 dark:border-slate-700 rounded">
          <h4 className="text-xs font-medium text-slate-700 dark:text-slate-300 mb-2">
            目标存储配置 ({formData.destType.toUpperCase()})
          </h4>
          {renderStorageConfig("dest", formData.destType)}
          {renderTestButton("目标", "dest", formData.destType)}
        </div>
      )}

      {formData.sourceType === "local" && (
        <div className="p-3 border border-blue-200 dark:border-blue-700 rounded bg-blue-50/50 dark:bg-blue-900/10">
          <h4 className="text-xs font-medium text-blue-700 dark:text-blue-300 mb-2">
            源本地路径
          </h4>
          <div className="flex gap-2">
            <input
              type="text"
              value={formData.sourceLocalPath}
              onChange={(e) =>
                setFormData({ ...formData, sourceLocalPath: e.target.value })
              }
              placeholder="C:\Users\YourName\Documents"
              className="flex-1 px-2 py-1.5 rounded border border-slate-300 dark:border-slate-600 bg-white dark:bg-slate-800 text-sm text-slate-900 dark:text-white focus:ring-1 focus:ring-blue-500 focus:border-blue-500 outline-none transition-all"
            />
            <button
              type="button"
              onClick={async () => {
                try {
                  const selected = await open({
                    directory: true,
                    multiple: false,
                    title: "选择源文件夹",
                  });
                  if (selected && typeof selected === "string") {
                    setFormData({ ...formData, sourceLocalPath: selected });
                  }
                } catch (err) {
                  console.error("选择文件夹失败:", err);
                }
              }}
              className="px-2 py-1.5 rounded border border-slate-300 dark:border-slate-600 bg-slate-50 dark:bg-slate-700 hover:bg-slate-100 dark:hover:bg-slate-600 transition-colors"
              title="选择文件夹"
            >
              <FolderOpen className="w-4 h-4 text-slate-600 dark:text-slate-400" />
            </button>
          </div>
        </div>
      )}
      {formData.destType === "local" && (
        <div className="p-3 border border-green-200 dark:border-green-700 rounded bg-green-50/50 dark:bg-green-900/10">
          <h4 className="text-xs font-medium text-green-700 dark:text-green-300 mb-2">
            目标本地路径
          </h4>
          <div className="flex gap-2">
            <input
              type="text"
              value={formData.destLocalPath}
              onChange={(e) =>
                setFormData({ ...formData, destLocalPath: e.target.value })
              }
              placeholder="D:\Backup"
              className="flex-1 px-2 py-1.5 rounded border border-slate-300 dark:border-slate-600 bg-white dark:bg-slate-800 text-sm text-slate-900 dark:text-white focus:ring-1 focus:ring-blue-500 focus:border-blue-500 outline-none transition-all"
            />
            <button
              type="button"
              onClick={async () => {
                try {
                  const selected = await open({
                    directory: true,
                    multiple: false,
                    title: "选择目标文件夹",
                  });
                  if (selected && typeof selected === "string") {
                    setFormData({ ...formData, destLocalPath: selected });
                  }
                } catch (err) {
                  console.error("选择文件夹失败:", err);
                }
              }}
              className="px-2 py-1.5 rounded border border-slate-300 dark:border-slate-600 bg-slate-50 dark:bg-slate-700 hover:bg-slate-100 dark:hover:bg-slate-600 transition-colors"
              title="选择文件夹"
            >
              <FolderOpen className="w-4 h-4 text-slate-600 dark:text-slate-400" />
            </button>
          </div>
        </div>
      )}
    </div>
  );

  const renderStorageConfig = (side: "source" | "dest", type: StorageType) => {
    const inputClass =
      "w-full px-2 py-1.5 rounded border border-slate-300 dark:border-slate-600 bg-white dark:bg-slate-800 text-sm focus:ring-1 focus:ring-blue-500 focus:border-blue-500 outline-none";
    const isSource = side === "source";

    if (type === "s3") {
      return (
        <div className="space-y-2">
          <input
            type="text"
            value={isSource ? formData.sourceS3Bucket : formData.destS3Bucket}
            onChange={(e) =>
              setFormData({ ...formData, [isSource ? "sourceS3Bucket" : "destS3Bucket"]: e.target.value })
            }
            placeholder="Bucket 名称"
            className={inputClass}
          />
          <input
            type="text"
            value={isSource ? formData.sourceS3Region : formData.destS3Region}
            onChange={(e) =>
              setFormData({ ...formData, [isSource ? "sourceS3Region" : "destS3Region"]: e.target.value })
            }
            placeholder="Region (如: us-east-1)"
            className={inputClass}
          />
          <input
            type="text"
            value={isSource ? formData.sourceS3AccessKey : formData.destS3AccessKey}
            onChange={(e) =>
              setFormData({ ...formData, [isSource ? "sourceS3AccessKey" : "destS3AccessKey"]: e.target.value })
            }
            placeholder="Access Key ID"
            className={inputClass}
          />
          <input
            type="password"
            value={isSource ? formData.sourceS3SecretKey : formData.destS3SecretKey}
            onChange={(e) =>
              setFormData({ ...formData, [isSource ? "sourceS3SecretKey" : "destS3SecretKey"]: e.target.value })
            }
            placeholder="Secret Access Key"
            className={inputClass}
          />
          <input
            type="text"
            value={isSource ? formData.sourceS3Endpoint : formData.destS3Endpoint}
            onChange={(e) =>
              setFormData({ ...formData, [isSource ? "sourceS3Endpoint" : "destS3Endpoint"]: e.target.value })
            }
            placeholder="Endpoint (可选，如 MinIO)"
            className={inputClass}
          />
        </div>
      );
    }

    if (type === "webdav") {
      return (
        <div className="space-y-2">
          <input
            type="text"
            value={isSource ? formData.sourceWebdavEndpoint : formData.destWebdavEndpoint}
            onChange={(e) =>
              setFormData({ ...formData, [isSource ? "sourceWebdavEndpoint" : "destWebdavEndpoint"]: e.target.value })
            }
            placeholder="https://dav.example.com"
            className={inputClass}
          />
          <input
            type="text"
            value={isSource ? formData.sourceWebdavUsername : formData.destWebdavUsername}
            onChange={(e) =>
              setFormData({ ...formData, [isSource ? "sourceWebdavUsername" : "destWebdavUsername"]: e.target.value })
            }
            placeholder="用户名"
            className={inputClass}
          />
          <input
            type="password"
            value={isSource ? formData.sourceWebdavPassword : formData.destWebdavPassword}
            onChange={(e) =>
              setFormData({ ...formData, [isSource ? "sourceWebdavPassword" : "destWebdavPassword"]: e.target.value })
            }
            placeholder="密码"
            className={inputClass}
          />
        </div>
      );
    }

    return null;
  };

  return (
    <div
      className={`fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/50 ${isClosing ? "dialog-overlay-out" : "dialog-overlay"}`}
    >
      <div
        className={`w-full max-w-lg bg-white dark:bg-slate-800 rounded shadow-xl ${isClosing ? "dialog-content-out" : "dialog-content"}`}
      >
        {/* 头部 */}
        <div className="flex items-center justify-between px-4 py-3 border-b border-slate-200 dark:border-slate-700">
          <h2 className="text-sm font-medium text-slate-900 dark:text-white">
            {isEditing ? (
              "编辑任务"
            ) : (
              <>
                {step === 1 && "创建同步任务"}
                {step === 2 && "选择存储类型"}
                {step === 3 && "配置存储"}
              </>
            )}
          </h2>
          <button
            onClick={() => handleClose()}
            className="p-1 rounded hover:bg-slate-100 dark:hover:bg-slate-700 transition-colors"
          >
            <X className="w-4 h-4 text-slate-500" />
          </button>
        </div>

        {/* 步骤指示（编辑模式不显示） */}
        {!isEditing && (
          <div className="flex items-center justify-center gap-2 py-3 px-4">
            {[1, 2, 3].map((s) => (
              <React.Fragment key={s}>
                <div
                  className={cn(
                    "w-6 h-6 rounded-full flex items-center justify-center text-xs font-medium transition-colors",
                    step >= s
                      ? "bg-blue-500 text-white"
                      : "bg-slate-200 dark:bg-slate-700 text-slate-500",
                  )}
                >
                  {s}
                </div>
                {s < 3 && (
                  <div
                    className={cn(
                      "w-8 h-0.5 rounded",
                      step > s
                        ? "bg-blue-500"
                        : "bg-slate-200 dark:bg-slate-700",
                    )}
                  />
                )}
              </React.Fragment>
            ))}
          </div>
        )}

        {/* 内容 */}
        <div className="px-4 py-3 max-h-[400px] overflow-y-auto">
          {isEditing ? (
            // 编辑模式：显示所有配置
            <div className="space-y-4">
              {/* 基本信息 */}
              <div>
                <label className="block text-xs font-medium text-slate-700 dark:text-slate-300 mb-1.5">
                  任务名称
                </label>
                <input
                  type="text"
                  value={formData.name}
                  onChange={(e) =>
                    setFormData({ ...formData, name: e.target.value })
                  }
                  className="w-full px-3 py-1.5 rounded border border-slate-300 dark:border-slate-600 bg-white dark:bg-slate-800 text-sm text-slate-900 dark:text-white focus:ring-1 focus:ring-blue-500 focus:border-blue-500 outline-none transition-all"
                />
              </div>
              {/* 同步模式 */}
              <div>
                <label className="block text-xs font-medium text-slate-700 dark:text-slate-300 mb-2">
                  同步模式
                </label>
                <div className="grid grid-cols-3 gap-2">
                  {(["bidirectional", "mirror", "backup"] as SyncMode[]).map((mode) => (
                    <button
                      key={mode}
                      type="button"
                      onClick={() => setFormData({ ...formData, syncMode: mode })}
                      className={cn(
                        "p-2.5 rounded border text-left transition-all",
                        formData.syncMode === mode
                          ? "border-blue-500 bg-blue-50 dark:bg-blue-900/20"
                          : "border-slate-200 dark:border-slate-700 hover:border-slate-300 dark:hover:border-slate-600",
                      )}
                    >
                      <div className="text-xs font-medium text-slate-900 dark:text-white">
                        {getSyncModeLabel(mode)}
                      </div>
                    </button>
                  ))}
                </div>
              </div>
              {/* 存储配置 */}
              {renderStep3()}
            </div>
          ) : (
            <>
              {step === 1 && renderStep1()}
              {step === 2 && renderStep2()}
              {step === 3 && renderStep3()}
            </>
          )}
        </div>

        {/* 底部按钮 */}
        <div className="flex items-center justify-between px-4 py-3 border-t border-slate-200 dark:border-slate-700">
          {isEditing ? (
            <button
              onClick={() => handleClose()}
              className="px-3 py-1.5 rounded text-sm text-slate-700 dark:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-700 transition-colors"
            >
              取消
            </button>
          ) : (
            <button
              onClick={() =>
                setStep(step === 1 ? 1 : ((step - 1) as 1 | 2 | 3))
              }
              disabled={step === 1}
              className={cn(
                "px-3 py-1.5 rounded text-sm transition-colors",
                step === 1
                  ? "text-slate-400 cursor-not-allowed"
                  : "text-slate-700 dark:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-700",
              )}
            >
              上一步
            </button>
          )}
          <div className="flex gap-2">
            {isEditing || step === 3 ? (
              <button
                onClick={handleSubmit}
                disabled={isCreating || !formData.name}
                className={cn(
                  "px-3 py-1.5 rounded text-sm transition-colors",
                  isCreating || !formData.name
                    ? "bg-slate-200 dark:bg-slate-700 text-slate-500 cursor-not-allowed"
                    : "bg-blue-500 hover:bg-blue-600 text-white",
                )}
              >
                {isCreating
                  ? isEditing
                    ? "保存中..."
                    : "创建中..."
                  : isEditing
                    ? "保存"
                    : "创建任务"}
              </button>
            ) : (
              <button
                onClick={() => setStep((step + 1) as 1 | 2 | 3)}
                disabled={step === 1 && !formData.name}
                className={cn(
                  "px-3 py-1.5 rounded text-sm transition-colors",
                  step === 1 && !formData.name
                    ? "bg-slate-200 dark:bg-slate-700 text-slate-500 cursor-not-allowed"
                    : "bg-blue-500 hover:bg-blue-600 text-white",
                )}
              >
                下一步
              </button>
            )}
          </div>
        </div>
      </div>

      {/* 消息弹窗 */}
      <MessageDialog
        isOpen={messageDialog.isOpen}
        title={messageDialog.title}
        message={messageDialog.message}
        type={messageDialog.type}
        onClose={() => setMessageDialog({ ...messageDialog, isOpen: false })}
      />
    </div>
  );
}
