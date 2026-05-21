import { useEffect } from "react";

export function usePoll(enabled: boolean, intervalMs: number, callback: () => void): void {
  useEffect(() => {
    if (!enabled) {
      return;
    }
    callback();
    const id = window.setInterval(callback, intervalMs);
    return () => window.clearInterval(id);
  }, [callback, enabled, intervalMs]);
}
