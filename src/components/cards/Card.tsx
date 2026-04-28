import type { PropsWithChildren, HTMLAttributes } from "react";

interface CardProps extends HTMLAttributes<HTMLDivElement> {
  hover?: boolean;
  padding?: "none" | "sm" | "md" | "lg";
}

const paddingStyles = {
  none: "",
  sm: "p-3",
  md: "p-4",
  lg: "p-6"
};

export function Card({
  hover = false,
  padding = "md",
  className = "",
  children,
  ...props
}: PropsWithChildren<CardProps>) {
  return (
    <div
      className={`bg-surface-800 border border-surface-700 rounded-xl ${
        paddingStyles[padding]
      } ${hover ? "hover:border-surface-500 transition-colors cursor-pointer" : ""} ${className}`}
      {...props}
    >
      {children}
    </div>
  );
}
