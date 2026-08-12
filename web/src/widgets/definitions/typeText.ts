import { TypeTextWidget } from "../../components/TypeTextWidget";
import { Icons } from "../../ui/icons/Icons";
import { box, chip } from "../preview";
import { WidgetDefinition, requiresPlugin } from "../types";

export const typeTextWidget: WidgetDefinition = {
  type: "type-text",
  label: "Type Text",
  description: "Type a string on the host",
  icon: Icons.typeText,
  category: "input",
  requires: [
    requiresPlugin("key-simulator", "Types the text into the focused window."),
  ],
  defaultSize: { colSpan: 1, rowSpan: 1 },
  defaultVariant: "compact",
  variants: [
    {
      value: "compact",
      label: "Mini",
      description: "Small button with a text preview",
      preview: () => box("simple-preview", chip("Hello!")),
    },
    {
      value: "detailed",
      label: "Full",
      description: "Large button with the full text",
      preview: () => box("simple-preview", chip("Hello, world!", true)),
    },
  ],
  settings: [
    {
      kind: "textarea",
      key: "text",
      label: "Text",
      default: "Hello!",
      placeholder: "Text to type...",
      rows: 3,
      required: true,
    },
  ],
  component: TypeTextWidget,
};
