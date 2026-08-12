/**
 * Field validators shared between definitions.
 *
 * Each returns `null` when the value is fine, or the message to show. They are
 * only ever called with a non-blank value — `required` handles emptiness — so
 * they can judge the value on its own terms.
 */

/** A URL the host or browser will actually accept. */
export function httpUrl(value: any): string | null {
  let parsed: URL;
  try {
    parsed = new URL(String(value));
  } catch {
    return "Enter a full URL, including https://";
  }
  if (parsed.protocol !== "http:" && parsed.protocol !== "https:") {
    return "Only http and https URLs can be opened";
  }
  return null;
}

/**
 * A URL the *proxy* will accept.
 *
 * The server refuses to fetch loopback and private addresses, so catching the
 * obvious cases here turns a runtime error into a message beside the field.
 * It is deliberately a hint, not a duplicate of the server's check: the
 * authoritative rule lives in `sd-api`, and DNS names cannot be judged here.
 */
export function proxyableUrl(value: any, settings: Record<string, any>): string | null {
  const invalid = httpUrl(value);
  if (invalid) return invalid;
  if (settings.mode !== "proxy") return null;

  const host = new URL(String(value)).hostname.toLowerCase();
  const isPrivate =
    host === "localhost" ||
    host === "0.0.0.0" ||
    host.endsWith(".localhost") ||
    /^127\./.test(host) ||
    /^10\./.test(host) ||
    /^192\.168\./.test(host) ||
    /^169\.254\./.test(host) ||
    /^172\.(1[6-9]|2\d|3[01])\./.test(host) ||
    host === "::1";

  return isPrivate
    ? "The server refuses to proxy private addresses. Use Local (browser) mode, or set SD_PROXY_ALLOW_PRIVATE=1 on the host."
    : null;
}

/** A whole number inside a range. */
export function intRange(min: number, max: number) {
  return (value: any): string | null => {
    const parsed = Number(value);
    if (!Number.isInteger(parsed)) return "Enter a whole number";
    if (parsed < min || parsed > max) return `Enter a number between ${min} and ${max}`;
    return null;
  };
}
