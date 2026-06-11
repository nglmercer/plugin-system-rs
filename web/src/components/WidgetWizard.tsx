import { h } from "preact";
import { useState } from "preact/hooks";
import { WidgetConfig, WIDGET_VARIANTS } from "../lib/types";
import { recordHotkey } from "../lib/api";
import { WidgetContent } from "./WidgetContent";

interface WidgetWizardProps {
  widget: WidgetConfig;
  columns: number;
  onSave: (id: string, updates: { title?: string; colSpan?: number; settings?: Record<string, any> }) => void;
  onRemove: () => void;
  onClose: () => void;
}

export function WidgetWizard({ widget, columns, onSave, onRemove, onClose }: WidgetWizardProps) {
  const [step, setStep] = useState(0);
  const [title, setTitle] = useState(widget.title);
  const [colSpan, setColSpan] = useState(widget.colSpan);
  const [settings, setSettings] = useState({ ...widget.settings });
  const [variant, setVariant] = useState<string>(widget.settings.variant || "compact");
  const totalSteps = 4;

  function handleNext() { if (step < totalSteps - 1) setStep(step + 1); }
  function handleBack() { if (step > 0) setStep(step - 1); }
  function handleApply() { onSave(widget.id, { title, colSpan, settings: { ...settings, variant } }); }

  return h("div", { class: "wizard-overlay", onClick: onClose },
    h("div", { class: "wizard-modal", onClick: (e: Event) => e.stopPropagation() },
      h("div", { class: "wizard-header" },
        h("div", { class: "wizard-title" }, `Edit: ${widget.type}`),
        h("button", { class: "picker-close", onClick: onClose }, "X"),
      ),
      h("div", { class: "wizard-steps" },
        ["General", "Config", "Style", "Apply"].map((label, i) =>
          h("div", { class: `wizard-step-indicator ${i === step ? "active" : i < step ? "done" : ""}`, key: label },
            h("div", { class: "wizard-step-circle" }, label[0]),
            h("div", { class: "wizard-step-label" }, label),
          )
        ),
      ),
      h("div", { class: "wizard-body" },
        step === 0 && h(WizardGeneral, { title, colSpan, columns, onChangeTitle: setTitle, onChangeColSpan: setColSpan }),
        step === 1 && h(WizardConfig, { widget, settings, onChange: setSettings }),
        step === 2 && h(WizardStyle, { widget, variant, onChange: setVariant }),
        step === 3 && h(WizardConfirm, { widget, title, colSpan, settings, variant, onApply: handleApply, onRemove }),
      ),
      h("div", { class: "wizard-footer" },
        step > 0 && h("button", { class: "wizard-btn back", onClick: handleBack }, "Back"),
        h("div", { class: "wizard-footer-spacer" }),
        step < totalSteps - 1
          ? h("button", { class: "wizard-btn next", onClick: handleNext }, "Next")
          : h("button", { class: "wizard-btn apply", onClick: handleApply }, "Save & Close"),
      ),
    ),
  );
}

function WizardGeneral({ title, colSpan, columns, onChangeTitle, onChangeColSpan }: {
  title: string; colSpan: number; columns: number;
  onChangeTitle: (t: string) => void; onChangeColSpan: (c: number) => void;
}) {
  return h("div", { class: "wizard-step-content" },
    h("h3", { class: "wizard-step-heading" }, "General Settings"),
    h("div", { class: "wizard-field" },
      h("label", null, "Widget Title"),
      h("input", { type: "text", value: title, onInput: (e: Event) => onChangeTitle((e.target as HTMLInputElement).value), placeholder: "Enter widget title..." }),
    ),
    h("div", { class: "wizard-field" },
      h("label", null, "Column Span"),
      h("input", { type: "number", min: "1", max: String(columns), value: String(colSpan), onInput: (e: Event) => onChangeColSpan(parseInt((e.target as HTMLInputElement).value) || 1) }),
      h("span", { class: "wizard-field-hint" }, `Grid has ${columns} columns`),
    ),
  );
}

function WizardConfig({ widget, settings, onChange }: {
  widget: WidgetConfig; settings: Record<string, any>; onChange: (s: Record<string, any>) => void;
}) {
  function set(key: string, value: any) { onChange({ ...settings, [key]: value }); }
  return h("div", { class: "wizard-step-content" },
    h("h3", { class: "wizard-step-heading" }, "Widget Configuration"),
    widget.type === "send-hotkey" && h(HotkeyRecorder, { currentKeys: settings.keys || "", onChange: (keys) => set("keys", keys) }),
    widget.type === "open-url" && h("div", { class: "wizard-field" },
      h("label", null, "URL"),
      h("input", { type: "text", value: settings.url || "", placeholder: "https://example.com", onInput: (e: Event) => set("url", (e.target as HTMLInputElement).value) }),
    ),
    widget.type === "type-text" && h("div", { class: "wizard-field" },
      h("label", null, "Text"),
      h("textarea", { value: settings.text || "", placeholder: "Text to type...", onInput: (e: Event) => set("text", (e.target as HTMLTextAreaElement).value) }),
    ),
    widget.type === "system-monitor" && h("div", { class: "wizard-field" },
      h("label", null, "Refresh Interval (ms)"),
      h("input", { type: "number", min: "500", step: "500", value: String(settings.refreshInterval || 2000), onInput: (e: Event) => set("refreshInterval", parseInt((e.target as HTMLInputElement).value) || 2000) }),
    ),
  );
}

function HotkeyRecorder({ currentKeys, onChange }: { currentKeys: string; onChange: (keys: string) => void }) {
  const [recording, setRecording] = useState(false);
  const [pending, setPending] = useState<string | null>(null);

  async function startRecording() {
    setRecording(true);
    setPending(null);
    try {
      const combo = await recordHotkey(3000);
      setPending(combo);
    } catch { setPending(null); }
    setRecording(false);
  }

  function confirmCombo() { if (pending) { onChange(pending); setPending(null); } }
  function cancelPending() { setPending(null); }

  return h("div", { class: "wizard-field" },
    h("label", null, "Hotkey Combination"),
    !pending && h("div", { class: "hotkey-display" },
      h("span", { class: `hotkey-keys ${recording ? "listening" : ""}` }, recording ? "Listening..." : currentKeys || "Not set"),
      h("button", { class: `hotkey-record-btn ${recording ? "recording" : ""}`, onClick: recording ? () => {} : startRecording }, recording ? "..." : "Record"),
    ),
    pending && h("div", { class: "hotkey-pending" },
      h("span", { class: "hotkey-pending-combo" }, pending),
      h("button", { class: "hotkey-confirm-btn", onClick: confirmCombo }, "Confirm"),
      h("button", { class: "hotkey-retry-btn", onClick: startRecording }, "Retry"),
      h("button", { class: "hotkey-cancel-btn", onClick: cancelPending }, "Cancel"),
    ),
  );
}

function WizardStyle({ widget, variant, onChange }: {
  widget: WidgetConfig; variant: string; onChange: (v: string) => void;
}) {
  const entries = WIDGET_VARIANTS.find((e) => e.type === widget.type);
  if (!entries) return null;

  return h("div", { class: "wizard-step-content" },
    h("h3", { class: "wizard-step-heading" }, "Style Variant"),
    h("p", { class: "wizard-step-desc" }, "Choose how this widget displays"),
    h("div", { class: "variant-grid" },
      entries.variants.map((v) =>
        h("button", { class: `variant-card ${variant === v.value ? "selected" : ""}`, key: v.value, onClick: () => onChange(v.value) },
          h("div", { class: "variant-card-preview" }, h(VariantPreview, { type: widget.type, variant: v.value })),
          h("div", { class: "variant-card-info" },
            h("div", { class: "variant-card-label" }, v.label),
            h("div", { class: "variant-card-desc" }, v.description),
          ),
        )
      ),
    ),
  );
}

function VariantPreview({ type, variant }: { type: string; variant: string }) {
  switch (type) {
    case "system-monitor":
      switch (variant) {
        case "minimal": return h("div", { class: "variant-preview sysmon-minimal" }, h("div", null, "42% CPU"), h("div", null, "56% RAM"));
        case "compact": return h("div", { class: "variant-preview sysmon-compact" },
          h("div", { class: "mini-bar" }, h("div", { class: "mini-bar-fill", style: { width: "42%", background: "#4caf50" } })),
          h("div", { class: "mini-bar" }, h("div", { class: "mini-bar-fill", style: { width: "56%", background: "#2196f3" } })),
        );
        case "detailed": return h("div", { class: "variant-preview sysmon-detailed" }, h("div", { class: "mini-grid" }, h("div", null, "42%"), h("div", null, "56%"), h("div", null, "1.2"), h("div", null, "2d")));
      }
    case "clock":
      switch (variant) {
        case "simple": return h("div", { class: "variant-preview clock-simple" }, "14:30");
        case "digital": return h("div", { class: "variant-preview clock-digital" }, "14:30", h("div", { class: "mini-sec" }, "15"), h("div", { class: "mini-date" }, "Mon"));
        case "detailed": return h("div", { class: "variant-preview clock-detailed" }, "14:30:15", h("div", { class: "mini-date" }, "Monday, Jun 10"));
      }
    default:
      return h("div", { class: "variant-preview simple-preview" },
        h("div", { class: variant === "compact" ? "preview-btn-sm" : "preview-btn-lg" }, "Action"),
      );
  }
}

function WizardConfirm({ widget, title, colSpan, settings, variant, onApply, onRemove }: {
  widget: WidgetConfig; title: string; colSpan: number; settings: Record<string, any>; variant: string;
  onApply: () => void; onRemove: () => void;
}) {
  return h("div", { class: "wizard-step-content" },
    h("h3", { class: "wizard-step-heading" }, "Confirm Changes"),
    h("p", { class: "wizard-step-desc" }, "Review your widget configuration before saving"),
    h("div", { class: "confirm-details" },
      h("div", { class: "confirm-row" }, h("span", null, "Title"), h("span", null, title)),
      h("div", { class: "confirm-row" }, h("span", null, "Span"), h("span", null, `${colSpan} column${colSpan > 1 ? "s" : ""}`)),
      h("div", { class: "confirm-row" }, h("span", null, "Variant"), h("span", null, variant)),
      ...Object.entries(settings).filter(([k]) => k !== "variant").map(([k, v]) =>
        h("div", { class: "confirm-row", key: k }, h("span", null, k), h("span", null, String(v).substring(0, 40)))
      ),
    ),
    h("div", { class: "confirm-preview" },
      h("div", { class: "wizard-step-heading", style: "font-size:0.8rem;color:#888;margin-bottom:0.5rem" }, "Preview"),
      h("div", { class: "preview-frame" }, h(WidgetContent, { widget: { ...widget, title, colSpan, settings: { ...settings, variant } } })),
    ),
    h("button", { class: "wizard-remove-btn", onClick: onRemove }, "Delete Widget"),
  );
}
