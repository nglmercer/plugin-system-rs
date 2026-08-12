/**
 * One shared view of which plugins the host has loaded.
 *
 * Several places need this at once — the library dims unavailable widgets, the
 * wizard shows a requirements panel, and every widget placeholder explains
 * itself — and they must agree. A per-component fetch would give three answers
 * and three `/api/plugins` requests every time a modal opened, so this is a
 * module-level cache with subscribers.
 */

import { useEffect, useState } from "preact/hooks";
import { fetchPlugins } from "../lib/api";
import { HostStatus, UNKNOWN_HOST_STATUS } from "./requirements";

/** How long a fetched status is reused before the next consumer refetches. */
const STALE_AFTER_MS = 5_000;

let status: HostStatus = UNKNOWN_HOST_STATUS;
let fetchedAt = 0;
let inFlight: Promise<void> | null = null;
const listeners = new Set<(status: HostStatus) => void>();

function publish(next: HostStatus) {
  status = next;
  listeners.forEach((listener) => listener(next));
}

/**
 * Fetch the plugin list unless a fresh copy is already in hand.
 *
 * Concurrent callers share one request: opening the wizard mounts several
 * consumers in the same tick, and three identical requests would be three
 * chances to disagree.
 */
export async function refreshHostStatus(force = false): Promise<HostStatus> {
  const fresh = Date.now() - fetchedAt < STALE_AFTER_MS;
  if (!force && fresh && status.loaded) return status;
  if (inFlight) {
    await inFlight;
    return status;
  }

  inFlight = (async () => {
    try {
      const plugins = await fetchPlugins();
      fetchedAt = Date.now();
      publish({ plugins, loaded: true });
    } catch {
      // Treat an unreachable server as "nothing loaded, and we know it".
      // Staying in the unloaded state would report every requirement as met
      // and hide exactly the problem the user is looking at.
      fetchedAt = Date.now();
      publish({ plugins: [], loaded: true });
    } finally {
      inFlight = null;
    }
  })();

  await inFlight;
  return status;
}

/** Drop the cache, so the next read refetches. Call after changing a plugin. */
export function invalidateHostStatus(): void {
  fetchedAt = 0;
}

/**
 * The last known status, without subscribing.
 *
 * For the handful of places that render outside a component — a custom settings
 * field, for instance — where a hook is not available.
 */
export function hostStatusSnapshot(): HostStatus {
  return status;
}

export function useHostStatus(): HostStatus {
  const [current, setCurrent] = useState(status);

  useEffect(() => {
    listeners.add(setCurrent);
    void refreshHostStatus();
    return () => {
      listeners.delete(setCurrent);
    };
  }, []);

  return current;
}
