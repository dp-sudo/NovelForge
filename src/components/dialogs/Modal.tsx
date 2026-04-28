import { type PropsWithChildren } from "react";
import * as Dialog from "@radix-ui/react-dialog";
import { cn } from "../../lib/utils.js";

interface ModalProps {
  open: boolean;
  onClose: () => void;
  title?: string;
  width?: "sm" | "md" | "lg";
}

const widthStyles = {
  sm: "max-w-sm",
  md: "max-w-lg",
  lg: "max-w-2xl",
};

export function Modal({ open, onClose, title, width = "md", children }: PropsWithChildren<ModalProps>) {
  return (
    <Dialog.Root open={open} onOpenChange={(open) => { if (!open) onClose(); }}>
      <Dialog.Portal>
        <Dialog.Overlay className="fixed inset-0 z-50 bg-black/60 data-[state=open]:animate-in data-[state=closed]:animate-out" />
        <Dialog.Content
          className={cn(
            "fixed left-1/2 top-1/2 z-50 -translate-x-1/2 -translate-y-1/2",
            "bg-card border border-border rounded-xl shadow-2xl",
            "w-full mx-4 max-h-[85vh] flex flex-col",
            "data-[state=open]:animate-in data-[state=closed]:animate-out",
            widthStyles[width],
          )}
        >
          {title && (
            <div className="flex items-center justify-between px-5 py-4 border-b border-border">
              <Dialog.Title className="text-base font-semibold text-foreground">
                {title}
              </Dialog.Title>
              <Dialog.Close className="text-surface-400 hover:text-surface-200 transition-colors text-lg leading-none rounded-sm focus:outline-none focus:ring-2 focus:ring-ring">
                ✕
              </Dialog.Close>
            </div>
          )}
          <div className="flex-1 overflow-y-auto px-5 py-4">{children}</div>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
