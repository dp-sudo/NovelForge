import { type InputHTMLAttributes, forwardRef } from "react";

interface InputProps extends InputHTMLAttributes<HTMLInputElement> {
  label?: string;
  error?: string;
  helperText?: string;
}

export const Input = forwardRef<HTMLInputElement, InputProps>(
  ({ label, error, helperText, className = "", id, ...props }, ref) => {
    const inputId = id ?? label?.toLowerCase().replace(/\s+/g, "-");

    return (
      <div className="flex flex-col gap-1.5">
        {label && (
          <label
            htmlFor={inputId}
            className="text-sm font-medium text-surface-200"
          >
            {label}
          </label>
        )}
        <input
          ref={ref}
          id={inputId}
          className={`px-3 py-2 text-sm bg-surface-800 border rounded-lg text-surface-100 placeholder-surface-400 transition-colors focus:outline-none focus:ring-2 focus:ring-primary/50 ${
            error
              ? "border-error focus:border-error"
              : "border-surface-600 focus:border-primary"
          } ${className}`}
          {...props}
        />
        {error && <span className="text-xs text-error">{error}</span>}
        {helperText && !error && (
          <span className="text-xs text-surface-400">{helperText}</span>
        )}
      </div>
    );
  }
);

Input.displayName = "Input";
