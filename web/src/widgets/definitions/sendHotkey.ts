import { SendHotkeyWidget } from "../../components/SendHotkeyWidget";
import { Icons } from "../../ui/icons/Icons";
import { box, chip } from "../preview";
import { WidgetDefinition, requiresPlugin } from "../types";

export const sendHotkeyWidget: WidgetDefinition = {
  type: "send-hotkey",
  label: "Send Hotkey",
  description: "Press a key combination on the host",
  icon: Icons.hotkey,
  category: "input",
  requires: [
    requiresPlugin("key-simulator", "Injects the key combination into the focused window."),
  ],
  defaultSize: { colSpan: 1, rowSpan: 1 },
  defaultVariant: "compact",
  variants: [
    {
      value: "compact",
      label: "Mini",
      description: "Small button with the hotkey",
      preview: () => box("simple-preview", chip("CTRL+C")),
    },
    {
      value: "detailed",
      label: "Full",
      description: "Large button with hotkey and label",
      preview: () => box("simple-preview", chip("CTRL + SHIFT + V", true)),
    },
  ],
  settings: [
    {
      kind: "hotkey",
      key: "keys",
      label: "Hotkey",
      hint: "Recorded or picked",
      default: "ctrl+c",
      required: true,
      // A widget whose whole job is to press a combination is not configured
      // until it has one, and saving it anyway produces a button that silently
      // does nothing.
      validate: (value) =>
        String(value).split("+").filter(Boolean).length === 0
          ? "Pick at least one key"
          : null,
    },
  ],
  component: SendHotkeyWidget,
};
