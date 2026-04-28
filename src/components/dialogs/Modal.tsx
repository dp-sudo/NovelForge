import { useEffect, type PropsWithChildren } from "react";

interface ModalProps {
  open: boolean;
  onClose: () => void;
  title?: string;
  width?: "sm" | "md" | "lg";
}

const widthStyles = {
  sm: "max-w-sm",
  md: "max-w-lg",
  lg: "max-w-2xl"
};

export function Modal({
  open,
  onClose,
  title,
  width = "md",
  children
}: PropsWithChildren<ModalProps>) {
  useEffect(() => {
    if (!open) return;
    function handleKey(e: KeyboardEvent) {
      if (e.key === "Escape") onClose();
    }
    document.addEventListener("keydown", handleKey);
    return () => document.removeEventListener("keydown", handleKey);
  }, [open, onClose]);

  if (!open) return null;

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60"
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div
        className={`bg-surface-800 border border-surface-700 rounded-xl shadow-2xl w-full ${widthStyles[width]} mx-4 max-h-[85vh] flex flex-col`}
      >
        {title && (
          <div className="flex items-center justify-between px-5 py-4 border-b border-surface-700">
            <h2 className="text-base font-semibold text-surface-100">
              {title}
            </h2>
            <button
              onClick={onClose}
              className="text-surface-400 hover:text-surface-200 transition-colors text-lg leading-none"
            >
              ✕
            </button>
          </div>
        )}
        <div className="flex-1 overflow-y-auto px-5 py-4">{children}</div>
      </div>
    </div>
  );
}
