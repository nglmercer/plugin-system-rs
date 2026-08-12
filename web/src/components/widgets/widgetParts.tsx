import { h, ComponentChildren } from "preact";
import { CSSCustomProperties } from "../../lib/types";

/**
 * Building blocks the deck widgets share.
 *
 * Before these existed, every widget carried its own copy of the same
 * loading box, the same slider gradient, the same mute button — each with its
 * own class prefix and its own hardcoded colours. One copy lives here now.
 */

/* ── States ─────────────────────────────────────────────── */

export function WidgetLoading() {
  return h("div", { class: "widget-state" }, "Loading…");
}

export function WidgetError({ children }: { children?: ComponentChildren }) {
  return h("div", { class: "widget-state error" }, children);
}

export function WidgetEmpty({ icon, children }: { icon?: string; children?: ComponentChildren }) {
  return h(
    "div",
    { class: "widget-empty" },
    icon ? h("div", { class: "widget-empty-icon" }, icon) : null,
    h("div", { class: "widget-empty-text" }, children),
  );
}

/* ── Section head ───────────────────────────────────────── */

/** The uppercase title row atop a widget's list, with an optional value. */
export function WidgetHead({ title, value }: { title: string; value?: string }) {
  return h(
    "div",
    { class: "widget-head" },
    h("span", { class: "widget-head-title" }, title),
    value ? h("span", { class: "widget-head-value" }, value) : null,
  );
}

/* ── Slider ─────────────────────────────────────────────── */

interface SliderRowProps {
  /** Current level, 0–100. */
  value: number;
  onInput: (value: number) => void;
  /** Fill colour; defaults to the slider's own accent. */
  fill?: string;
  /** For fractional ranges (OBS reports volume as 0–1). */
  fraction?: boolean;
}

/**
 * Slider plus its percentage readout.
 *
 * The fill is painted with a gradient driven by two custom properties set
 * inline — `--sd-slider-pct` and `--sd-slider-fill` — so the track shows the
 * level without any per-widget CSS of its own.
 */
export function SliderRow({ value, onInput, fill, fraction }: SliderRowProps) {
  const pct = Math.round(fraction ? value * 100 : value);
  return h(
    "div",
    { class: "widget-slider-row" },
    h("input", {
      type: "range",
      class: "widget-slider",
      min: 0,
      max: fraction ? 1 : 100,
      step: fraction ? 0.01 : 1,
      value,
      onInput: (e: Event) => onInput(parseFloat((e.target as HTMLInputElement).value)),
      style: {
        "--sd-slider-pct": `${pct}%`,
        ...(fill ? { "--sd-slider-fill": fill } : {}),
      } as CSSCustomProperties,
    }),
    h("span", { class: "widget-slider-value" }, `${pct}%`),
  );
}

/* ── Mute button ────────────────────────────────────────── */

interface MuteButtonProps {
  muted: boolean;
  onToggle: (muted: boolean) => void;
  /** `icon` is the small square; `pill` the labelled MUTE/UNMUTE button. */
  kind?: "icon" | "pill";
  /** Letters shown unmuted/muted in icon mode, e.g. ["V", "M"]. */
  labels?: [string, string];
}

export function MuteButton({ muted, onToggle, kind = "icon", labels = ["V", "M"] }: MuteButtonProps) {
  if (kind === "pill") {
    return h(
      "button",
      {
        class: `widget-mute-pill${muted ? " muted" : ""}`,
        type: "button",
        onClick: () => onToggle(!muted),
      },
      muted ? "UNMUTE" : "MUTE",
    );
  }
  return h(
    "button",
    {
      class: `widget-mute${muted ? " muted" : ""}`,
      type: "button",
      title: muted ? "Unmute" : "Mute",
      onClick: () => onToggle(!muted),
    },
    muted ? labels[1] : labels[0],
  );
}
