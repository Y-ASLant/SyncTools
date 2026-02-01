import React, { useState, useEffect } from "react";
import {
  X,
  Folder,
  Cloud,
  Server,
  Check,
  X as XIcon,
  Loader2,
  FolderOpen,
} from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { cn } from "../lib/utils";
import { MessageDialog } from "./MessageDialog";
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
  s3Bucket: string;
  s3Region: string;
  s3AccessKey: string;
  s3SecretKey: string;
  s3Endpoint: string;
  webdavEndpoint: string;
  webdavUsername: string;
  webdavPassword: string;
  localPath: string;
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
    s3Bucket: "",
    s3Region: "us-east-1",
    s3AccessKey: "",
    s3SecretKey: "",
    s3Endpoint: "",
    webdavEndpoint: "",
    webdavUsername: "",
    webdavPassword: "",
    localPath: "",
  });

  const [isCreating, setIsCreating] = useState(false);
  const [testing, setTesting] = useState<Record<string, boolean>>({});
  const [testResults, setTestResults] = useState<
    Record<string, TestConnectionResult>
  >({});
  const [visible, setVisible] = useState(false);
  const [isClosing, setIsClosing] = useState(false);
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

  useEffect(() => {
    if (isOpen) {
      setVisible(true);
      setIsClosing(false);
    }
  }, [isOpen]);

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
        s3Bucket:
          editJob.sourceConfig.bucket || editJob.destConfig.bucket || "",
        s3Region:
          editJob.sourceConfig.region ||
          editJob.destConfig.region ||
          "us-east-1",
        s3AccessKey:
          editJob.sourceConfig.accessKey || editJob.destConfig.accessKey || "",
        s3SecretKey:
          editJob.sourceConfig.secretKey || editJob.destConfig.secretKey || "",
        s3Endpoint:
          editJob.sourceConfig.endpoint || editJob.destConfig.endpoint || "",
        webdavEndpoint:
          editJob.sourceConfig.webdavEndpoint ||
          editJob.destConfig.webdavEndpoint ||
          "",
        webdavUsername:
          editJob.sourceConfig.username || editJob.destConfig.username || "",
        webdavPassword:
          editJob.sourceConfig.password || editJob.destConfig.password || "",
        localPath: editJob.sourceConfig.path || editJob.destConfig.path || "",
      });
      setStep(3); // 编辑模式直接跳到配置页
    }
  }, [editJob]);

  if (!visible) return null;

  const testConnection = async (storage: "source" | "dest") => {
    const type = storage === "source" ? formData.sourceType : formData.destType;
    const key = `${storage}-${type}`;

    setTesting((prev) => ({ ...prev, [key]: true }));
    setTestResults((prev) => ({
      ...prev,
      [key]: { success: false, message: "测试中...", details: null },
    }));

    try {
      const result = await invoke<TestConnectionResult>("test_connection", {
        typ: type,
        path: formData.localPath || null,
        bucket: formData.s3Bucket || null,
        region: formData.s3Region || null,
        accessKey: formData.s3AccessKey || null,
        secretKey: formData.s3SecretKey || null,
        endpoint: formData.s3Endpoint || null,
        webdavEndpoint: formData.webdavEndpoint || null,
        username: formData.webdavUsername || null,
        password: formData.webdavPassword || null,
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
      if (!formData.localPath.trim()) {
        return "请输入本地路径";
      }
    } else if (formData.sourceType === "s3") {
      if (!formData.s3Bucket.trim()) return "请输入 S3 Bucket 名称";
      if (!formData.s3Region.trim()) return "请输入 S3 Region";
      if (!formData.s3AccessKey.trim()) return "请输入 Access Key ID";
      if (!formData.s3SecretKey.trim()) return "请输入 Secret Access Key";
    } else if (formData.sourceType === "webdav") {
      if (!formData.webdavEndpoint.trim()) return "请输入 WebDAV 地址";
      if (!formData.webdavUsername.trim()) return "请输入 WebDAV 用户名";
      if (!formData.webdavPassword.trim()) return "请输入 WebDAV 密码";
    }

    // 验证目标存储配置
    if (formData.destType === "local") {
      if (!formData.localPath.trim()) {
        return "请输入本地路径";
      }
    } else if (formData.destType === "s3") {
      if (!formData.s3Bucket.trim()) return "请输入 S3 Bucket 名称";
      if (!formData.s3Region.trim()) return "请输入 S3 Region";
      if (!formData.s3AccessKey.trim()) return "请输入 Access Key ID";
      if (!formData.s3SecretKey.trim()) return "请输入 Secret Access Key";
    } else if (formData.destType === "webdav") {
      if (!formData.webdavEndpoint.trim()) return "请输入 WebDAV 地址";
      if (!formData.webdavUsername.trim()) return "请输入 WebDAV 用户名";
      if (!formData.webdavPassword.trim()) return "请输入 WebDAV 密码";
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
      const buildStorageConfig = (type: StorageType) => {
        switch (type) {
          case "local":
            return { type: "local", path: formData.localPath };
          case "s3":
            return {
              type: "s3",
              bucket: formData.s3Bucket,
              region: formData.s3Region,
              accessKey: formData.s3AccessKey,
              secretKey: formData.s3SecretKey,
              endpoint: formData.s3Endpoint || undefined,
            };
          case "webdav":
            return {
              type: "webdav",
              webdavEndpoint: formData.webdavEndpoint,
              username: formData.webdavUsername,
              password: formData.webdavPassword,
            };
        }
      };

      if (isEditing && editJob) {
        // 编辑模式：更新任务
        await invoke("update_job", {
          id: editJob.id,
          name: formData.name,
          sourceConfig: buildStorageConfig(formData.sourceType),
          destConfig: buildStorageConfig(formData.destType),
          syncMode: formData.syncMode,
        });
      } else {
        // 创建模式：新建任务
        await invoke("create_job", {
          name: formData.name,
          sourceConfig: buildStorageConfig(formData.sourceType),
          destConfig: buildStorageConfig(formData.destType),
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

  const handleClose = () => {
    setIsClosing(true);
    setTimeout(() => {
      setStep(1);
      setFormData({
        name: "",
        sourceType: "local",
        destType: "s3",
        syncMode: "backup",
        s3Bucket: "",
        s3Region: "us-east-1",
        s3AccessKey: "",
        s3SecretKey: "",
        s3Endpoint: "",
        webdavEndpoint: "",
        webdavUsername: "",
        webdavPassword: "",
        localPath: "",
      });
      setTestResults({});
      setVisible(false);
      onClose();
    }, 100);
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
                {mode === "bidirectional"
                  ? "双向同步"
                  : mode === "mirror"
                    ? "镜像同步"
                    : "备份"}
              </div>
              <div className="text-xs text-slate-500 dark:text-slate-400">
                {mode === "bidirectional"
                  ? "两边修改都会同步"
                  : mode === "mirror"
                    ? "目标完全复制源"
                    : "只从源同步到目标"}
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
                {type === "local" ? "本地" : type === "s3" ? "S3" : "WebDAV"}
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
                {type === "local" ? "本地" : type === "s3" ? "S3" : "WebDAV"}
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
              <XIcon className="w-3 h-3" />
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

      {(formData.sourceType === "local" || formData.destType === "local") && (
        <div className="p-3 border border-slate-200 dark:border-slate-700 rounded">
          <h4 className="text-xs font-medium text-slate-700 dark:text-slate-300 mb-2">
            本地路径
          </h4>
          <div className="flex gap-2">
            <input
              type="text"
              value={formData.localPath}
              onChange={(e) =>
                setFormData({ ...formData, localPath: e.target.value })
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
                    title: "选择文件夹",
                  });
                  if (selected && typeof selected === "string") {
                    setFormData({ ...formData, localPath: selected });
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
          <p className="mt-1.5 text-xs text-slate-500 dark:text-slate-400">
            点击右侧按钮浏览文件夹，或直接输入路径
          </p>
        </div>
      )}
    </div>
  );

  const renderStorageConfig = (_side: "source" | "dest", type: StorageType) => {
    const inputClass =
      "w-full px-2 py-1.5 rounded border border-slate-300 dark:border-slate-600 bg-white dark:bg-slate-800 text-sm focus:ring-1 focus:ring-blue-500 focus:border-blue-500 outline-none";

    if (type === "s3") {
      return (
        <div className="space-y-2">
          <input
            type="text"
            value={formData.s3Bucket}
            onChange={(e) =>
              setFormData({ ...formData, s3Bucket: e.target.value })
            }
            placeholder="Bucket 名称"
            className={inputClass}
          />
          <input
            type="text"
            value={formData.s3Region}
            onChange={(e) =>
              setFormData({ ...formData, s3Region: e.target.value })
            }
            placeholder="Region (如: us-east-1)"
            className={inputClass}
          />
          <input
            type="text"
            value={formData.s3AccessKey}
            onChange={(e) =>
              setFormData({ ...formData, s3AccessKey: e.target.value })
            }
            placeholder="Access Key ID"
            className={inputClass}
          />
          <input
            type="password"
            value={formData.s3SecretKey}
            onChange={(e) =>
              setFormData({ ...formData, s3SecretKey: e.target.value })
            }
            placeholder="Secret Access Key"
            className={inputClass}
          />
          <input
            type="text"
            value={formData.s3Endpoint}
            onChange={(e) =>
              setFormData({ ...formData, s3Endpoint: e.target.value })
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
            value={formData.webdavEndpoint}
            onChange={(e) =>
              setFormData({ ...formData, webdavEndpoint: e.target.value })
            }
            placeholder="https://dav.example.com"
            className={inputClass}
          />
          <input
            type="text"
            value={formData.webdavUsername}
            onChange={(e) =>
              setFormData({ ...formData, webdavUsername: e.target.value })
            }
            placeholder="用户名"
            className={inputClass}
          />
          <input
            type="password"
            value={formData.webdavPassword}
            onChange={(e) =>
              setFormData({ ...formData, webdavPassword: e.target.value })
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
            onClick={handleClose}
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
              onClick={handleClose}
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
