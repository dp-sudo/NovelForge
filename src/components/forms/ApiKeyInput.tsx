import { useState } from "react";

interface ApiKeyInputProps {
  label?: string;
  value: string;
  onChange: (value: string) => void;
  maskedValue?: string;
  error?: string;
}

export function ApiKeyInput({
  label = "API Key",
  value,
  onChange,
  maskedValue,
  error
}: ApiKeyInputProps) {
  const [showKey, setShowKey] = useState(false);
  const [isEditing, setIsEditing] = useState(false);
  const hasExistingKey = !!maskedValue && !value;

  return (
    <div className="flex flex-col gap-1.5">
      {label && (
        <label className="text-sm font-medium text-surface-200">{label}</label>
      )}

      {hasExistingKey && !isEditing ? (
        <div className="flex items-center gap-2 px-3 py-2 text-sm bg-surface-800 border border-surface-600 rounded-lg">
          <span className="text-surface-300 flex-1">{maskedValue}</span>
          <button
            type="button"
            onClick={() => setIsEditing(true)}
            className="text-xs text-primary hover:text-primary-light"
          >
            替换
          </button>
        </div>
      ) : (
        <div className="relative">
          <input
            type={showKey ? "text" : "password"}
            value={value}
            onChange={(e) => onChange(e.target.value)}
            placeholder={maskedValue ? "输入新 Key 替换" : "sk-..."}
            className={`w-full px-3 py-2 pr-20 text-sm bg-surface-800 border rounded-lg text-surface-100 placeholder-surface-400 transition-colors focus:outline-none focus:ring-2 focus:ring-primary/50 ${
              error ? "border-error" : "border-surface-600 focus:border-primary"
            }`}
          />
          <div className="absolute right-2 top-1/2 -translate-y-1/2 flex gap-1">
            <button
              type="button"
              onClick={() => setShowKey(!showKey)}
              className="px-2 py-1 text-xs text-surface-400 hover:text-surface-200"
            >
              {showKey ? "隐藏" : "显示"}
            </button>
            {value && (
              <button
                type="button"
                onClick={() => onChange("")}
                className="px-2 py-1 text-xs text-error hover:text-error-light"
              >
                清除
              </button>
            )}
          </div>
        </div>
      )}

      {error && <span className="text-xs text-error">{error}</span>}
      {!error && (
        <span className="text-xs text-surface-500">
          {hasExistingKey
            ? "API Key 已保存，点击「替换」可更新"
            : "API Key 将安全保存在 Windows Credential Manager"}
        </span>
      )}
    </div>
  );
}
