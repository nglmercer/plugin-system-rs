/**
 * API token handling for the dashboard.
 *
 * The daemon requires a bearer token on every `/api` and `/ws` request. The
 * dashboard obtains it one of three ways, in order:
 *
 *  1. `?token=` in the address bar — how a phone gets it after scanning the QR
 *     code, since a remote browser cannot use the loopback bootstrap.
 *  2. `localStorage`, from a previous visit.
 *  3. `GET /api/auth/token`, which the server answers for loopback callers
 *     only. This is the normal path on the machine running the daemon.
 *
 * Rather than threading the token through the ~60 call sites in `api.ts` and
 * the widgets, `installFetchInterceptor` wraps `window.fetch` and attaches the
 * header to same-origin API requests. That deliberately excludes the arbitrary
 * URLs `FetchWidget` fetches in direct mode — sending our credential to a
 * third-party host would be a worse bug than the one this fixes.
 */

const STORAGE_KEY = "sd.apiToken";

let token: string | null = null;
let interceptorInstalled = false;

export function getToken(): string | null {
  return token;
}

export function setToken(value: string | null) {
  token = value && value.length > 0 ? value : null;
  try {
    if (token) localStorage.setItem(STORAGE_KEY, token);
    else localStorage.removeItem(STORAGE_KEY);
  } catch {
    // Private browsing modes reject writes. An in-memory token still works
    // for this session, so this is not worth failing over.
  }
}

/**
 * An API URL with the token in the query string.
 *
 * For URLs the browser fetches on its own — `<img src>`, `<a download>`, a
 * `WebSocket` — which never reach the `fetch` wrapper and cannot carry a
 * header. Prefer the header everywhere else: a token in a URL ends up in logs
 * and history.
 */
export function authedUrl(path: string): string {
  if (!token) return path;
  const separator = path.includes("?") ? "&" : "?";
  return `${path}${separator}token=${encodeURIComponent(token)}`;
}

/** Whether a request should carry our credential. */
function isApiRequest(url: string): boolean {
  let parsed: URL;
  try {
    parsed = new URL(url, window.location.href);
  } catch {
    return false;
  }
  if (parsed.origin !== window.location.origin) return false;
  return parsed.pathname === "/ws" || parsed.pathname.startsWith("/api");
}

function installFetchInterceptor() {
  if (interceptorInstalled) return;
  interceptorInstalled = true;

  const original = window.fetch.bind(window);

  window.fetch = (input: RequestInfo | URL, init?: RequestInit) => {
    const url =
      typeof input === "string"
        ? input
        : input instanceof URL
          ? input.href
          : input.url;

    if (!token || !isApiRequest(url)) {
      return original(input, init);
    }

    const headers = new Headers(init?.headers ?? (input instanceof Request ? input.headers : undefined));
    headers.set("Authorization", `Bearer ${token}`);
    return original(input, { ...init, headers });
  };
}

/** Pull a token out of the query string and scrub it from the address bar. */
function consumeTokenFromLocation(): string | null {
  const params = new URLSearchParams(window.location.search);
  const fromUrl = params.get("token");
  if (!fromUrl) return null;

  // Leaving the secret in the URL means it survives in history, bookmarks and
  // any screenshot of the browser. It is stored now, so drop it.
  params.delete("token");
  const query = params.toString();
  window.history.replaceState(
    {},
    "",
    `${window.location.pathname}${query ? `?${query}` : ""}${window.location.hash}`,
  );
  return fromUrl;
}

/**
 * Resolve a token and install the interceptor.
 *
 * Returns false when no token could be obtained, which means the dashboard is
 * being viewed from another device and the user has to supply one.
 */
export async function initAuth(): Promise<boolean> {
  const fromUrl = consumeTokenFromLocation();
  if (fromUrl) {
    setToken(fromUrl);
  } else {
    try {
      token = localStorage.getItem(STORAGE_KEY);
    } catch {
      token = null;
    }
  }

  installFetchInterceptor();

  if (token && (await tokenWorks())) return true;

  // A stored token that no longer works (the daemon regenerated it) is worse
  // than none: it produces 401s the user cannot explain. Drop it and retry the
  // loopback bootstrap.
  setToken(null);

  try {
    const res = await fetch("/api/auth/token");
    const data = await res.json();
    if (res.ok && data.success && data.data?.token) {
      setToken(data.data.token);
      return true;
    }
  } catch {
    // Server unreachable. Treat it the same as "no token": the entry screen
    // explains what to do, and a reload retries.
  }

  return false;
}

/** Cheapest authenticated call available, used to validate a stored token. */
async function tokenWorks(): Promise<boolean> {
  try {
    const res = await fetch("/api/devices");
    return res.status !== 401;
  } catch {
    return false;
  }
}
