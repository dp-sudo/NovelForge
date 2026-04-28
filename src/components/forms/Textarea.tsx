import { forwardRef, type TextareaHTMLAttributes } from "react";
import { cn } from "../../lib/utils.js";

interface TextareaProps extends TextareaHTMLAttributes<HTMLTextAreaElement> {
  label?: string;
  error?: string;
  helperText?: string;
}

export const Textarea = forwardRef<HTMLTextAreaElement, TextareaProps>(
  ({ label, error, helperText, className, id, ...props }, ref) => {
    const textareaId = id ?? label?.toLowerCase().replace(/\s+/g, "-");

    return (
      <div className="flex flex-col gap-1.5">
        {label && (
          <label htmlFor={textareaId} className="text-sm font-medium text-surface-200">
            {label}
          </label>
        )}
        <textarea
          ref={ref}
          id={textareaId}
          className={cn(
            "flex min-h-[80px] w-full rounded-lg border border-input bg-surface-800 px-3 py-2 text-sm text-surface-100 placeholder:text-surface-400 transition-colors",
            "focus:outline-none focus:ring-2 focus:ring-ring/50 focus:border-primary",
            "resize-y",
            error && "border-error",
            className,
          )}
          {...props}
        />
        {error && <span className="text-xs text-error">{error}</span>}
        {helperText && !error && <span className="text-xs text-surface-400">{helperText}</span>}
      </div>
    );
  },
);

Textarea.displayName = "Textarea";
