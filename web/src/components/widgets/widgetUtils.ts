/**
 * Small helpers shared by the deck widgets.
 */

/** Poll period a widget falls back to when its settings do not set one. */
export const DEFAULT_REFRESH_MS = 2000;

export function refreshMs(settings: Record<string, any>): number {
  return Number(settings.refreshInterval) || DEFAULT_REFRESH_MS;
}

/**
 * The colour for a measurement that goes green → amber → red as it climbs
 * (CPU load, volume, memory). Expressed as theme tokens so both modes stay
 * legible, instead of the raw hex literals this replaces.
 */
export function levelColor(value: number): string {
  if (value < 50) return "var(--sd-success)";
  if (value < 80) return "var(--sd-warning)";
  return "var(--sd-error)";
}

/** A percentage from a 0–1 fraction, for APIs that report volume that way. */
export function pctOf(fraction: number): number {
  return Math.round(fraction * 100);
}
