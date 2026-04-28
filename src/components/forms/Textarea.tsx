import { type TextareaHTMLAttributes, forwardRef } from "react";

interface TextareaProps extends TextareaHTMLAttributes<HTMLTextAreaElement> {
  label?: string;
  error?: string;
}

export const Textarea = forwardRef<HTMLTextAreaElement, TextareaProps>(
  ({ label, error, className = "", id, ...props }, ref) => {
    const textareaId = id ?? label?.toLowerCase().replace(/\s+/g, "-");

    return (
      <div className="flex flex-col gap-1.5">
        {label && (
          <label
            htmlFor={textareaId}
            className="text-sm font-medium text-surface-200"
          >
            {label}
          </label>
        )}
        <textarea
          ref={ref}
          id={textareaId}
          className={`px-3 py-2 text-sm bg-surface-800 border rounded-lg text-surface-100 placeholder-surface-400 transition-colors focus:outline-none focus:ring-2 focus:ring-primary/50 min-h-[80px] resize-y ${
            error
              ? "border-error focus:border-error"
              : "border-surface-600 focus:border-primary"
          } ${className}`}
          {...props}
        />
        {error && <span className="text-xs text-error">{error}</span>}
      </div>
    );
  }
);

Textarea.displayName = "Textarea";
