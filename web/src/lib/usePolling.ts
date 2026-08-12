import { useEffect, useRef } from "preact/hooks";

/** Longest gap between polls once a source keeps failing. */
export const MAX_POLL_BACKOFF_MS = 30_000;

/**
 * Compute the next delay after `failures` consecutive failures.
 *
 * A widget polling a source that is simply not there — OBS closed, for
 * instance — used to keep hammering the endpoint every two seconds forever.
 * Doubling the gap keeps the widget responsive when things work and quiet when
 * they do not, while still recovering on its own once the source comes back.
 */
export function pollDelay(baseMs: number, failures: number): number {
  if (failures <= 0) return baseMs;
  const scaled = baseMs * 2 ** Math.min(failures, 10);
  // A widget configured to poll more slowly than the cap keeps its own pace;
  // backing off must never speed a source up.
  return Math.min(scaled, Math.max(baseMs, MAX_POLL_BACKOFF_MS));
}

/**
 * Run `poll` on an interval, backing off while it keeps throwing.
 *
 * `poll` should throw to signal failure; the backoff resets on the first call
 * that returns normally.
 */
export function usePolling(poll: () => Promise<void>, baseMs: number) {
  const pollRef = useRef(poll);
  pollRef.current = poll;

  useEffect(() => {
    let cancelled = false;
    let timer: ReturnType<typeof setTimeout> | undefined;
    let failures = 0;

    const tick = async () => {
      try {
        await pollRef.current();
        failures = 0;
      } catch {
        failures += 1;
      }
      if (cancelled) return;
      timer = setTimeout(tick, pollDelay(baseMs, failures));
    };

    tick();

    return () => {
      cancelled = true;
      if (timer !== undefined) clearTimeout(timer);
    };
  }, [baseMs]);
}
