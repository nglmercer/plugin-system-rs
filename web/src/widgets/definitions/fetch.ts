import { FetchWidget } from "../../components/FetchWidget";
import { Icons } from "../../ui/icons/Icons";
import { box, text } from "../preview";
import { WidgetDefinition } from "../types";
import { proxyableUrl } from "../validators";

/** Methods that can carry a request body. */
const BODY_METHODS = ["POST", "PUT", "PATCH"];

export const fetchWidget: WidgetDefinition = {
  type: "fetch",
  label: "Fetch Data",
  description: "Call an HTTP endpoint and show the result",
  icon: Icons.fetch,
  category: "web",
  defaultSize: { colSpan: 1, rowSpan: 1 },
  defaultVariant: "compact",
  variants: [
    {
      value: "minimal",
      label: "Minimal",
      description: "Just the status code or a brief value",
      preview: () => box("fetch-minimal", text("200", "mini-status ok")),
    },
    {
      value: "compact",
      label: "Compact",
      description: "Status, URL and a small preview",
      preview: () =>
        box("fetch-compact", text("api.ex...", "mini-url"), text('{"id":1...}', "mini-data")),
    },
    {
      value: "detailed",
      label: "Detailed",
      description: "Full response body preview",
      preview: () =>
        box(
          "fetch-detailed",
          text("https://api.example.com/v1", "mini-url"),
          text('{\n  "status": "ok"\n}', "mini-json"),
        ),
    },
  ],
  settings: [
    {
      kind: "url",
      key: "url",
      label: "URL",
      default: "",
      placeholder: "https://api.example.com/data",
      required: true,
      validate: proxyableUrl,
    },
    {
      kind: "select",
      key: "mode",
      label: "Fetch mode",
      hint: "Who makes the request",
      default: "proxy",
      options: [
        { value: "proxy", label: "Proxy (backend)" },
        { value: "local", label: "Local (browser)" },
      ],
    },
    {
      kind: "select",
      key: "method",
      label: "HTTP method",
      default: "GET",
      options: ["GET", "POST", "PUT", "PATCH", "DELETE"].map((value) => ({
        value,
        label: value,
      })),
    },
    {
      kind: "keyvalue",
      key: "headers",
      label: "Headers",
      default: "",
      placeholder: { key: "Authorization", value: "Bearer ..." },
    },
    {
      kind: "select",
      key: "bodyType",
      label: "Body format",
      default: "json",
      options: [
        { value: "json", label: "JSON" },
        { value: "form", label: "Form encoded" },
        { value: "raw", label: "Raw" },
      ],
      visibleWhen: (settings) => BODY_METHODS.includes(settings.method || "GET"),
    },
    {
      kind: "textarea",
      key: "body",
      label: "Request body",
      default: "",
      rows: 4,
      placeholder: '{ "key": "value" }',
      // Only methods that can carry a body get the field, so a GET widget is
      // not offered a control the request will discard.
      visibleWhen: (settings) => BODY_METHODS.includes(settings.method || "GET"),
      validate: (value, settings) => {
        if (settings.bodyType !== "json") return null;
        try {
          JSON.parse(String(value));
          return null;
        } catch (e) {
          return `Body is not valid JSON: ${(e as Error).message}`;
        }
      },
    },
    {
      kind: "select",
      key: "fetchMode",
      label: "When to fetch",
      default: "auto",
      options: [
        { value: "auto", label: "On an interval" },
        { value: "once", label: "Only when clicked" },
      ],
    },
    {
      // Seconds, not milliseconds: this is the key `FetchWidget` reads, and a
      // schema that invented `refreshInterval` here would have produced a
      // control that silently changed nothing.
      kind: "number",
      key: "intervalSec",
      label: "Refresh interval",
      hint: "seconds",
      default: 30,
      min: 1,
      visibleWhen: (settings) => (settings.fetchMode ?? "auto") === "auto",
    },
  ],
  component: FetchWidget,
};
