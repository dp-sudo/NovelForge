import { type PropsWithChildren, useRef } from "react";

/**
 * Wraps page content with a fade-in-up animation on mount.
 * Uses a key-based re-mount to trigger re-animation when the key changes.
 */
export function PageTransition({ children, routeKey }: PropsWithChildren<{ routeKey: string }>) {
  const prevKey = useRef(routeKey);

  return (
    <div key={routeKey} className="animate-fade-in-up">
      {children}
    </div>
  );
}

/** Skeleton placeholder for loading states. */
export function Skeleton({ className = "" }: { className?: string }) {
  return (
    <div
      className={`animate-skeleton rounded-lg ${className}`}
      aria-hidden="true"
    />
  );
}

/** Card-shaped skeleton for list loading. */
export function CardSkeleton({ count = 3 }: { count?: number }) {
  return (
    <div className="space-y-3">
      {Array.from({ length: count }).map((_, i) => (
        <Skeleton key={i} className="h-20 w-full" />
      ))}
    </div>
  );
}

/** Stat card skeleton for dashboard loading. */
export function StatSkeleton() {
  return (
    <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-6 gap-4">
      {Array.from({ length: 6 }).map((_, i) => (
        <Skeleton key={i} className="h-24" />
      ))}
    </div>
  );
}
