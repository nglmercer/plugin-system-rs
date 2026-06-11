import { h } from "preact";
import { WidgetConfig } from "../lib/types";
import { SystemMonitorWidget } from "./SystemMonitorWidget";
import { ClockWidget } from "./ClockWidget";
import { QuickActionsWidget } from "./QuickActionsWidget";
import { SendHotkeyWidget } from "./SendHotkeyWidget";
import { OpenUrlWidget } from "./OpenUrlWidget";
import { TypeTextWidget } from "./TypeTextWidget";
import { VolumeWidget } from "./VolumeWidget";
import { VolumeAppsWidget } from "./VolumeAppsWidget";

export function WidgetContent({ widget }: { widget: WidgetConfig }) {
  switch (widget.type) {
    case "system-monitor": return h(SystemMonitorWidget, { settings: widget.settings });
    case "clock": return h(ClockWidget, { settings: widget.settings });
    case "quick-actions": return h(QuickActionsWidget, null);
    case "send-hotkey": return h(SendHotkeyWidget, { settings: widget.settings });
    case "open-url": return h(OpenUrlWidget, { settings: widget.settings });
    case "type-text": return h(TypeTextWidget, { settings: widget.settings });
    case "volume-master": return h(VolumeWidget, { settings: widget.settings });
    case "volume-apps": return h(VolumeAppsWidget, { settings: widget.settings });
    default: return h("div", { class: "widget-unknown" }, "Unknown widget");
  }
}
