import { h } from "preact";
import { useState } from "preact/hooks";

/**
 * An application icon, resolved by the host from a freedesktop icon name.
 *
 * The plugin reports a *name* (`"firefox"`), not image bytes; the host looks
 * it up in the system icon theme and serves it from `/api/icon/:name`. Plenty
 * of applications set no usable icon name, and plenty of names resolve to
 * nothing, so a missing icon is an ordinary outcome rather than an error — the
 * component falls back to an initial on a colour derived from the app name.
 */

interface AppIconProps {
  /** Freedesktop icon name. Empty or unresolvable falls back to an initial. */
  icon?: string;
  /** Application name, used for the fallback initial and its colour. */
  name: string;
  /** Rendered edge length in px. */
  size?: number;
}

/**
 * Pick a stable colour for a name.
 *
 * Deterministic so an app keeps the same colour between refreshes, and spread
 * around the hue circle so adjacent entries in a list stay distinguishable.
 */
function hueFor(name: string): number {
  let hash = 0;
  for (let i = 0; i < name.length; i++) {
    hash = (hash * 31 + name.charCodeAt(i)) | 0;
  }
  return Math.abs(hash) % 360;
}

function initialOf(name: string): string {
  const trimmed = name.trim();
  return trimmed ? trimmed[0].toUpperCase() : "?";
}

export function AppIcon({ icon, name, size = 24 }: AppIconProps) {
  const [failed, setFailed] = useState(false);

  const box = {
    width: `${size}px`,
    height: `${size}px`,
    minWidth: `${size}px`,
  };

  if (!icon || failed) {
    const hue = hueFor(name);
    return h(
      "div",
      {
        class: "app-icon app-icon-fallback",
        style: {
          ...box,
          background: `hsl(${hue} 45% 28%)`,
          color: `hsl(${hue} 85% 78%)`,
          fontSize: `${Math.round(size * 0.5)}px`,
        },
        title: name,
        "aria-hidden": "true",
      },
      initialOf(name),
    );
  }

  return h("img", {
    class: "app-icon",
    style: box,
    src: `/api/icon/${encodeURIComponent(icon)}`,
    alt: "",
    loading: "lazy",
    // A 404 is expected for apps with no themed icon; swap to the initial.
    onError: () => setFailed(true),
  });
}
