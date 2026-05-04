import { useState, useCallback } from "react";

export interface UseAiTaskReturn<T = string> {
  loading: boolean;
  result: T | null;
  error: string | null;
  run: (fn: () => Promise<T>) => Promise<T | null>;
  reset: () => void;
}

/**
 * Shared hook for AI task execution across all pages.
 * Encapsulates the loading / result / error state triple and try-catch-finally flow.
 */
export function useAiTask<T = string>(): UseAiTaskReturn<T> {
  const [loading, setLoading] = useState(false);
  const [result, setResult] = useState<T | null>(null);
  const [error, setError] = useState<string | null>(null);

  const run = useCallback(async (fn: () => Promise<T>): Promise<T | null> => {
    setLoading(true);
    setResult(null);
    setError(null);
    try {
      const value = await fn();
      setResult(value);
      return value;
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : String(err);
      setError(message);
      return null;
    } finally {
      setLoading(false);
    }
  }, []);

  const reset = useCallback(() => {
    setResult(null);
    setError(null);
  }, []);

  return { loading, result, error, run, reset };
}
