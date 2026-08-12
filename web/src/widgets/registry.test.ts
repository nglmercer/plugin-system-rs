import { describe, expect, it, beforeEach } from "vitest";
import { h } from "preact";
import {
  getWidget,
  listWidgets,
  registerWidget,
  registryVersion,
  subscribeToRegistry,
  unregisterWidget,
  widgetsByCategory,
} from "./registry";
import { BUILTIN_WIDGETS, registerBuiltinWidgets } from "./index";
import { defaultSettings, validateSettings, visibleFields, WidgetDefinition } from "./types";
import { checkRequirements, summarizeUnmet } from "./requirements";
import { wizardStepIds } from "./wizardSteps";
import { validateDefinition } from "./external";

const stub = (overrides: Partial<WidgetDefinition> = {}): WidgetDefinition => ({
  type: "test-widget",
  label: "Test",
  description: "",
  icon: () => h("span", null, "T"),
  category: "system",
  defaultSize: { colSpan: 1, rowSpan: 1 },
  component: () => h("div", null),
  ...overrides,
});

describe("registry", () => {
  beforeEach(() => {
    registerBuiltinWidgets();
    unregisterWidget("test-widget");
  });

  it("registers and finds a widget", () => {
    registerWidget(stub());
    expect(getWidget("test-widget")?.label).toBe("Test");
  });

  it("reports an unknown type as absent rather than guessing", () => {
    expect(getWidget("no-such-widget")).toBeUndefined();
  });

  it("notifies subscribers so a late registration reaches mounted views", () => {
    let calls = 0;
    const unsubscribe = subscribeToRegistry(() => {
      calls += 1;
    });
    const before = registryVersion();

    registerWidget(stub());

    expect(calls).toBe(1);
    expect(registryVersion()).toBeGreaterThan(before);
    unsubscribe();
  });

  /** A widget in an unknown category must stay reachable, not vanish. */
  it("files a widget with an unrecognised category under plugins", () => {
    registerWidget(stub({ category: "made-up" as any }));
    const groups = widgetsByCategory();
    const plugins = groups.find((group) => group.category === "plugin");
    expect(plugins?.widgets.some((w) => w.type === "test-widget")).toBe(true);
  });
});

describe("built-in widgets", () => {
  beforeEach(() => registerBuiltinWidgets());

  it("registers every built-in", () => {
    for (const widget of BUILTIN_WIDGETS) {
      expect(getWidget(widget.type), widget.type).toBeDefined();
    }
    expect(listWidgets().length).toBeGreaterThanOrEqual(BUILTIN_WIDGETS.length);
  });

  /**
   * The whole point of the registry: a widget cannot exist half-way. Before,
   * a type could be in the catalog but missing from the render switch, or have
   * variants with no preview.
   */
  it("gives every built-in the pieces the UI needs", () => {
    for (const widget of BUILTIN_WIDGETS) {
      expect(typeof widget.component, `${widget.type} component`).toBe("function");
      expect(typeof widget.icon, `${widget.type} icon`).toBe("function");
      expect(widget.label, `${widget.type} label`).toBeTruthy();
      expect(widget.description, `${widget.type} description`).toBeTruthy();
      expect(widget.defaultSize.colSpan, `${widget.type} colSpan`).toBeGreaterThan(0);
      expect(widget.defaultSize.rowSpan, `${widget.type} rowSpan`).toBeGreaterThan(0);

      for (const variant of widget.variants ?? []) {
        expect(variant.preview, `${widget.type}/${variant.value} preview`).toBeTypeOf("function");
        expect(variant.description, `${widget.type}/${variant.value} desc`).toBeTruthy();
      }
    }
  });

  it("starts every built-in at settings that already validate", () => {
    for (const widget of BUILTIN_WIDGETS) {
      const settings = defaultSettings(widget);
      const errors = validateSettings(widget, settings);
      // Fetch is the deliberate exception: a URL cannot be defaulted, so the
      // wizard is meant to stop on it.
      if (widget.type === "fetch" || widget.type === "plugin-data") {
        expect(Object.keys(errors).length, widget.type).toBeGreaterThan(0);
      } else {
        expect(errors, widget.type).toEqual({});
      }
    }
  });

  it("names a default variant that actually exists", () => {
    for (const widget of BUILTIN_WIDGETS) {
      if (!widget.defaultVariant) continue;
      const values = (widget.variants ?? []).map((v) => v.value);
      expect(values, widget.type).toContain(widget.defaultVariant);
    }
  });
});

describe("settings schema", () => {
  it("collects defaults, including the variant", () => {
    const definition = stub({
      defaultVariant: "compact",
      variants: [{ value: "compact", label: "C", description: "" }],
      settings: [
        { kind: "text", key: "name", label: "Name", default: "hi" },
        { kind: "number", key: "size", label: "Size", default: 3 },
      ],
    });

    expect(defaultSettings(definition)).toEqual({ name: "hi", size: 3, variant: "compact" });
  });

  it("rejects a blank required field", () => {
    const definition = stub({
      settings: [{ kind: "text", key: "url", label: "URL", default: "", required: true }],
    });
    expect(validateSettings(definition, { url: "" })).toEqual({ url: "URL is required" });
    expect(validateSettings(definition, { url: "x" })).toEqual({});
  });

  /** A field the user cannot see must not be able to block them. */
  it("ignores hidden fields entirely", () => {
    const definition = stub({
      settings: [
        { kind: "select", key: "method", label: "Method", default: "GET", options: [] },
        {
          kind: "text",
          key: "body",
          label: "Body",
          default: "",
          required: true,
          visibleWhen: (s) => s.method === "POST",
        },
      ],
    });

    expect(visibleFields(definition, { method: "GET" })).toHaveLength(1);
    expect(validateSettings(definition, { method: "GET", body: "" })).toEqual({});
    expect(validateSettings(definition, { method: "POST", body: "" })).toHaveProperty("body");
  });

  /** `validate` judges a supplied value; it must never see a blank one. */
  it("does not run a custom validator on a blank optional field", () => {
    let called = false;
    const definition = stub({
      settings: [
        {
          kind: "text",
          key: "path",
          label: "Path",
          default: "",
          validate: () => {
            called = true;
            return "nope";
          },
        },
      ],
    });

    expect(validateSettings(definition, { path: "" })).toEqual({});
    expect(called).toBe(false);
  });
});

describe("requirements", () => {
  const definition = stub({
    requires: [{ kind: "plugin", plugin: "obs", reason: "Talks to OBS." }],
  });

  it("passes when the plugin is loaded", () => {
    const report = checkRequirements(definition, {
      loaded: true,
      plugins: [{ name: "obs", path: "", loaded: true, enabled: true, version: "1" }],
    });
    expect(report.satisfied).toBe(true);
  });

  /**
   * The three failures must be distinguishable. Conflating them is what made
   * "not connected to OBS" the only thing a user ever saw.
   */
  it("tells missing, disabled and failed-to-load apart", () => {
    const missing = checkRequirements(definition, { loaded: true, plugins: [] });
    expect(missing.unmet[0].status).toBe("Not installed");

    const disabled = checkRequirements(definition, {
      loaded: true,
      plugins: [{ name: "obs", path: "", loaded: false, enabled: false, version: "1" }],
    });
    expect(disabled.unmet[0].status).toBe("Disabled");

    const failed = checkRequirements(definition, {
      loaded: true,
      plugins: [{ name: "obs", path: "", loaded: false, enabled: true, version: "1" }],
    });
    expect(failed.unmet[0].status).toBe("Not loaded");

    for (const report of [missing, disabled, failed]) {
      expect(report.unmet[0].remedy, report.unmet[0].status).toBeTruthy();
    }
  });

  /** Flashing a false "missing" on every open trains people to ignore it. */
  it("assumes satisfied while the host status is still loading", () => {
    const report = checkRequirements(definition, { loaded: false, plugins: [] });
    expect(report.satisfied).toBe(true);
  });

  it("summarises unmet requirements for a tile", () => {
    const report = checkRequirements(definition, { loaded: true, plugins: [] });
    expect(summarizeUnmet(report)).toBe("Needs the obs plugin");
  });

  it("says nothing when everything is satisfied", () => {
    expect(summarizeUnmet({ results: [], unmet: [], satisfied: true })).toBe("");
  });
});

describe("external definitions", () => {
  it("accepts a minimal valid definition", () => {
    expect(validateDefinition(stub())).toBeNull();
  });

  /** A malformed contribution must be refused here, not crash inside the grid. */
  it("rejects the shapes that would break rendering", () => {
    expect(validateDefinition(null)).toBeTruthy();
    expect(validateDefinition({ label: "x", component: () => null })).toBeTruthy();
    expect(validateDefinition({ type: "x", label: "x" })).toBeTruthy();
    expect(validateDefinition({ type: "x", component: () => null })).toBeTruthy();
    expect(
      validateDefinition({ type: "x", label: "x", component: () => null, settings: {} }),
    ).toBeTruthy();
  });
});

describe("wizard steps", () => {
  beforeEach(() => registerBuiltinWidgets());

  /** The bug this replaced: every widget got every step, empty or not. */
  it("gives a widget with nothing to configure only title and review", () => {
    expect(wizardStepIds(stub())).toEqual(["general", "apply"]);
  });

  it("adds a requirements step only when the widget needs the host", () => {
    expect(wizardStepIds(stub())).not.toContain("requirements");
    expect(
      wizardStepIds(stub({ requires: [{ kind: "plugin", plugin: "obs", reason: "x" }] })),
    ).toContain("requirements");
  });

  it("adds a config step only when there are fields", () => {
    expect(wizardStepIds(stub())).not.toContain("config");
    expect(
      wizardStepIds(
        stub({ settings: [{ kind: "text", key: "a", label: "A", default: "" }] }),
      ),
    ).toContain("config");
  });

  /** One variant is not a choice, so it does not deserve a step. */
  it("adds a style step only when there is more than one variant", () => {
    const one = stub({ variants: [{ value: "a", label: "A", description: "" }] });
    expect(wizardStepIds(one)).not.toContain("style");

    const two = stub({
      variants: [
        { value: "a", label: "A", description: "" },
        { value: "b", label: "B", description: "" },
      ],
    });
    expect(wizardStepIds(two)).toContain("style");
  });

  it("keeps the steps in a stable order", () => {
    const everything = stub({
      requires: [{ kind: "plugin", plugin: "obs", reason: "x" }],
      settings: [{ kind: "text", key: "a", label: "A", default: "" }],
      variants: [
        { value: "a", label: "A", description: "" },
        { value: "b", label: "B", description: "" },
      ],
    });
    expect(wizardStepIds(everything)).toEqual([
      "requirements",
      "general",
      "config",
      "style",
      "apply",
    ]);
  });

  /** No shipped widget should present a step with nothing in it. */
  it("never gives a built-in an empty step", () => {
    for (const widget of BUILTIN_WIDGETS) {
      const steps = wizardStepIds(widget);
      if (steps.includes("config")) {
        expect(widget.settings!.length, `${widget.type} config`).toBeGreaterThan(0);
      }
      if (steps.includes("style")) {
        expect(widget.variants!.length, `${widget.type} style`).toBeGreaterThan(1);
      }
      if (steps.includes("requirements")) {
        expect(widget.requires!.length, `${widget.type} requires`).toBeGreaterThan(0);
      }
    }
  });
});
