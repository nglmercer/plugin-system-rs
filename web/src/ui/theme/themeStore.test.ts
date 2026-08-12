import { beforeEach, describe, expect, it } from "vitest";
import {
  ACCENT_STORAGE_KEY,
  LEGACY_MODE_STORAGE_KEY,
  MODE_STORAGE_KEY,
  isValidAccent,
  isValidMode,
  getResolvedTheme,
  getThemeState,
  initTheme,
  resetThemeStoreForTesting,
  resolveMode,
  setThemeAccent,
  setThemeMode,
  subscribeTheme,
  toggleThemeMode,
} from "./themeStore";

const storage = new Map<string, string>();
const attributes: Record<string, string> = {};
let mediaMatches = false;
let mediaListener: ((event: { matches: boolean }) => void) | null = null;

(globalThis as any).localStorage = {
  getItem: (key: string) => storage.get(key) ?? null,
  setItem: (key: string, value: string) => {
    storage.set(key, value);
  },
};

(globalThis as any).document = {
  documentElement: {
    setAttribute: (name: string, value: string) => {
      attributes[name] = value;
    },
  },
};

(globalThis as any).window = {
  matchMedia: (query: string) => ({
    matches: mediaMatches,
    media: query,
    addEventListener: (_type: string, fn: (event: { matches: boolean }) => void) => {
      mediaListener = fn;
    },
    removeEventListener: () => {
      mediaListener = null;
    },
  }),
};

function resetEnvironment() {
  storage.clear();
  for (const key of Object.keys(attributes)) delete attributes[key];
  mediaMatches = false;
  mediaListener = null;
  resetThemeStoreForTesting();
}

describe("resolveMode", () => {
  it("returns explicit modes unchanged", () => {
    expect(resolveMode("dark", true)).toBe("dark");
    expect(resolveMode("light", true)).toBe("light");
    expect(resolveMode("dark", false)).toBe("dark");
  });

  it("follows the system preference in system mode", () => {
    expect(resolveMode("system", true)).toBe("dark");
    expect(resolveMode("system", false)).toBe("light");
  });
});

describe("validation", () => {
  it("accepts only known modes", () => {
    expect(isValidMode("dark")).toBe(true);
    expect(isValidMode("light")).toBe(true);
    expect(isValidMode("system")).toBe(true);
    expect(isValidMode("auto")).toBe(false);
    expect(isValidMode(null)).toBe(false);
  });

  it("accepts only known accents", () => {
    expect(isValidAccent("cyan")).toBe(true);
    expect(isValidAccent("purple")).toBe(true);
    expect(isValidAccent("magenta")).toBe(false);
    expect(isValidAccent(42)).toBe(false);
  });
});

describe("theme store", () => {
  beforeEach(resetEnvironment);

  it("defaults to dark mode with the cyan accent", () => {
    initTheme();
    expect(getThemeState().mode).toBe("dark");
    expect(getThemeState().accent).toBe("cyan");
    expect(attributes["data-theme"]).toBe("dark");
    expect(attributes["data-accent"]).toBe("cyan");
  });

  it("restores persisted mode and accent", () => {
    storage.set(MODE_STORAGE_KEY, "light");
    storage.set(ACCENT_STORAGE_KEY, "orange");
    initTheme();
    expect(attributes["data-theme"]).toBe("light");
    expect(attributes["data-accent"]).toBe("orange");
  });

  it("reads the legacy 'theme' key when the new key is absent", () => {
    storage.set(LEGACY_MODE_STORAGE_KEY, "light");
    initTheme();
    expect(attributes["data-theme"]).toBe("light");
  });

  it("prefers the new key over the legacy key", () => {
    storage.set(LEGACY_MODE_STORAGE_KEY, "light");
    storage.set(MODE_STORAGE_KEY, "dark");
    initTheme();
    expect(attributes["data-theme"]).toBe("dark");
  });

  it("ignores invalid stored values", () => {
    storage.set(MODE_STORAGE_KEY, "neon");
    storage.set(ACCENT_STORAGE_KEY, "rainbow");
    initTheme();
    expect(getThemeState().mode).toBe("dark");
    expect(getThemeState().accent).toBe("cyan");
  });

  it("persists mode changes and updates attributes", () => {
    initTheme();
    setThemeMode("light");
    expect(storage.get(MODE_STORAGE_KEY)).toBe("light");
    expect(attributes["data-theme"]).toBe("light");
    expect(getResolvedTheme()).toBe("light");
  });

  it("toggles against the resolved theme", () => {
    initTheme();
    toggleThemeMode();
    expect(getResolvedTheme()).toBe("light");
    toggleThemeMode();
    expect(getResolvedTheme()).toBe("dark");
  });

  it("toggle from system mode resolves against the OS preference", () => {
    mediaMatches = true;
    initTheme();
    setThemeMode("system");
    expect(getResolvedTheme()).toBe("dark");
    toggleThemeMode();
    expect(getThemeState().mode).toBe("light");
  });

  it("persists accent changes", () => {
    initTheme();
    setThemeAccent("purple");
    expect(storage.get(ACCENT_STORAGE_KEY)).toBe("purple");
    expect(attributes["data-accent"]).toBe("purple");
  });

  it("tracks live system preference changes while in system mode", () => {
    initTheme();
    setThemeMode("system");
    expect(attributes["data-theme"]).toBe("light");

    let notified = 0;
    const unsubscribe = subscribeTheme(() => {
      notified += 1;
    });

    mediaMatches = true;
    mediaListener?.({ matches: true });

    expect(attributes["data-theme"]).toBe("dark");
    expect(notified).toBe(1);
    unsubscribe();
  });
});
