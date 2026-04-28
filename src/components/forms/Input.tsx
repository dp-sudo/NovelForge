import { forwardRef, type InputHTMLAttributes } from "react";
import { cn } from "../../lib/utils.js";

interface InputProps extends InputHTMLAttributes<HTMLInputElement> {
  label?: string;
  error?: string;
  helperText?: string;
  containerClassName?: string;
}

export const Input = forwardRef<HTMLInputElement, InputProps>(
  ({ label, error, helperText, containerClassName, className, id, ...props }, ref) => {
    const inputId = id ?? label?.toLowerCase().replace(/\s+/g, "-");

    return (
      <div className={cn("flex flex-col gap-1.5", containerClassName)}>
        {label && (
          <label htmlFor={inputId} className="text-sm font-medium text-surface-200">
            {label}
          </label>
        )}
        <input
          ref={ref}
          id={inputId}
          className={cn(
            "flex h-9 w-full rounded-lg border bg-surface-800 px-3 py-2 text-sm text-surface-100 placeholder:text-surface-400 transition-colors",
            "focus:outline-none focus:ring-2 focus:ring-ring/50 focus:border-primary",
            error ? "border-error" : "border-input",
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

Input.displayName = "Input";
