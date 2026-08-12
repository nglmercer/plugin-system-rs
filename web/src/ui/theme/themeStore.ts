/**
 * Theme store: mode (dark/light/system) + accent preset, persisted to
 * localStorage and applied as `data-theme` / `data-accent` attributes on
 * `<html>`, where the token stylesheets in ui/themes/ pick them up.
 */

export type ThemeMode = "dark" | "light" | "system";
export type ResolvedTheme = "dark" | "light";

export const ACCENTS = ["cyan", "orange", "green", "purple", "red"] as const;
export type AccentId = (typeof ACCENTS)[number];

export const MODE_STORAGE_KEY = "sd-theme";
export const ACCENT_STORAGE_KEY = "sd-theme-accent";
export const LEGACY_MODE_STORAGE_KEY = "theme";

const DARK_MEDIA_QUERY = "(prefers-color-scheme: dark)";

export function isValidMode(value: unknown): value is ThemeMode {
  return value === "dark" || value === "light" || value === "system";
}

export function isValidAccent(value: unknown): value is AccentId {
  return typeof value === "string" && (ACCENTS as readonly string[]).includes(value);
}

export function resolveMode(mode: ThemeMode, systemPrefersDark: boolean): ResolvedTheme {
  if (mode === "system") return systemPrefersDark ? "dark" : "light";
  return mode;
}

export interface ThemeState {
  mode: ThemeMode;
  accent: AccentId;
  systemPrefersDark: boolean;
}

let state: ThemeState | null = null;
const listeners = new Set<() => void>();
let mediaQuery: MediaQueryList | null = null;

function hasWindow(): boolean {
  return typeof window !== "undefined";
}

function readStored(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}

function writeStored(key: string, value: string): void {
  try {
    localStorage.setItem(key, value);
  } catch {
    // Storage unavailable (private mode, sandboxed iframe): theme still
    // applies for the current session.
  }
}

function systemPrefersDark(): boolean {
  if (!hasWindow() || typeof window.matchMedia !== "function") return false;
  return window.matchMedia(DARK_MEDIA_QUERY).matches;
}

function initState(): ThemeState {
  const storedMode = readStored(MODE_STORAGE_KEY) ?? readStored(LEGACY_MODE_STORAGE_KEY);
  const storedAccent = readStored(ACCENT_STORAGE_KEY);
  return {
    mode: isValidMode(storedMode) ? storedMode : "dark",
    accent: isValidAccent(storedAccent) ? storedAccent : "cyan",
    systemPrefersDark: systemPrefersDark(),
  };
}

function getState(): ThemeState {
  if (state === null) state = initState();
  return state;
}

function emit(): void {
  for (const listener of listeners) listener();
}

function applyAttributes(): void {
  if (!hasWindow()) return;
  const resolved = resolveMode(getState().mode, getState().systemPrefersDark);
  document.documentElement.setAttribute("data-theme", resolved);
  document.documentElement.setAttribute("data-accent", getState().accent);
}

function handleMediaChange(event: MediaQueryListEvent): void {
  state = { ...getState(), systemPrefersDark: event.matches };
  applyAttributes();
  emit();
}

function ensureMediaListener(): void {
  if (!hasWindow() || typeof window.matchMedia !== "function" || mediaQuery) return;
  mediaQuery = window.matchMedia(DARK_MEDIA_QUERY);
  mediaQuery.addEventListener("change", handleMediaChange);
}

export function getThemeState(): ThemeState {
  return getState();
}

export function getResolvedTheme(): ResolvedTheme {
  const s = getState();
  return resolveMode(s.mode, s.systemPrefersDark);
}

export function subscribeTheme(listener: () => void): () => void {
  listeners.add(listener);
  ensureMediaListener();
  return () => {
    listeners.delete(listener);
  };
}

export function setThemeMode(mode: ThemeMode): void {
  state = { ...getState(), mode };
  writeStored(MODE_STORAGE_KEY, mode);
  applyAttributes();
  emit();
}

export function setThemeAccent(accent: AccentId): void {
  state = { ...getState(), accent };
  writeStored(ACCENT_STORAGE_KEY, accent);
  applyAttributes();
  emit();
}

/** Switches between dark and light, matching the legacy toggle behavior. */
export function toggleThemeMode(): void {
  setThemeMode(getResolvedTheme() === "dark" ? "light" : "dark");
}

/** Applies the persisted theme before first render to avoid a flash. */
export function initTheme(): void {
  getState();
  applyAttributes();
}

/** Test hook: drops cached state and listeners. */
export function resetThemeStoreForTesting(): void {
  state = null;
  listeners.clear();
  if (mediaQuery) {
    mediaQuery.removeEventListener("change", handleMediaChange);
    mediaQuery = null;
  }
}
