import { type PropsWithChildren, type ReactElement, type ReactNode } from "react";
import * as Dialog from "@radix-ui/react-dialog";
import { cn } from "../../lib/utils.js";

interface ModalProps {
  open: boolean;
  onClose: () => void;
  title?: string;
  description?: string;
  width?: "sm" | "md" | "lg";
}

const widthStyles = {
  sm: "max-w-sm",
  md: "max-w-lg",
  lg: "max-w-2xl",
};

const DialogOverlay = Dialog.Overlay as unknown as (props: {
  children?: ReactNode;
  className?: string;
  asChild?: boolean;
}) => ReactElement;
const DialogDescription = Dialog.Description as unknown as (props: {
  children?: ReactNode;
  className?: string;
  asChild?: boolean;
}) => ReactElement;
const DialogTitle = Dialog.Title as unknown as (props: {
  children?: ReactNode;
  className?: string;
  asChild?: boolean;
}) => ReactElement;
const DialogClose = Dialog.Close as unknown as (props: {
  children?: ReactNode;
  className?: string;
  asChild?: boolean;
}) => ReactElement;

export function Modal({ open, onClose, title, description, width = "md", children }: PropsWithChildren<ModalProps>) {
  const a11yDescription = description ?? (title ? `${title}对话框内容` : "对话框内容");
  return (
    <Dialog.Root open={open} onOpenChange={(open) => { if (!open) onClose(); }}>
      <Dialog.Portal>
        <DialogOverlay asChild>
          <div className="fixed inset-0 z-50 bg-black/60 animate-fade-in" />
        </DialogOverlay>
        <Dialog.Content
          className={cn(
            "fixed left-1/2 top-1/2 z-50 -translate-x-1/2 -translate-y-1/2",
            "bg-card border border-border rounded-xl shadow-2xl",
            "w-full mx-4 max-h-[85vh] flex flex-col",
            "animate-scale-in",
            widthStyles[width],
          )}
        >
          <DialogDescription asChild>
            <p className="sr-only">{a11yDescription}</p>
          </DialogDescription>
          {title && (
            <div className="flex items-center justify-between px-5 py-4 border-b border-border">
              <DialogTitle asChild>
                <h2 className="text-base font-semibold text-foreground">{title}</h2>
              </DialogTitle>
              <DialogClose asChild>
                <button
                  type="button"
                  className="text-surface-400 hover:text-surface-200 transition-colors text-lg leading-none rounded-sm focus:outline-none focus:ring-2 focus:ring-ring"
                >
                  ✕
                </button>
              </DialogClose>
            </div>
          )}
          <div className="flex-1 overflow-y-auto px-5 py-4">{children}</div>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
