import { OpenUrlWidget } from "../../components/widgets/OpenUrlWidget";
import { Icons } from "../../ui/icons/Icons";
import { box, chip } from "../preview";
import { WidgetDefinition } from "../types";
import { httpUrl } from "../validators";

export const openUrlWidget: WidgetDefinition = {
  type: "open-url",
  label: "Open URL",
  description: "Open a link in the host's browser",
  icon: Icons.url,
  category: "web",
  defaultSize: { colSpan: 1, rowSpan: 1 },
  defaultVariant: "compact",
  variants: [
    {
      value: "compact",
      label: "Mini",
      description: "Small button with the URL",
      preview: () => box("simple-preview", chip("example.com")),
    },
    {
      value: "detailed",
      label: "Full",
      description: "Large button with a URL preview",
      preview: () => box("simple-preview", chip("https://example.com", true)),
    },
  ],
  settings: [
    {
      kind: "url",
      key: "url",
      label: "URL",
      default: "https://example.com",
      placeholder: "https://example.com",
      required: true,
      validate: httpUrl,
    },
  ],
  component: OpenUrlWidget,
};
