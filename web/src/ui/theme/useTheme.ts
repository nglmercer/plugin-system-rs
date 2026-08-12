import { useEffect, useState } from "preact/hooks";
import {
  getResolvedTheme,
  getThemeState,
  setThemeAccent,
  setThemeMode,
  subscribeTheme,
  toggleThemeMode,
} from "./themeStore";
import type { AccentId, ResolvedTheme, ThemeMode } from "./themeStore";

export interface ThemeApi {
  mode: ThemeMode;
  accent: AccentId;
  resolved: ResolvedTheme;
  setMode: (mode: ThemeMode) => void;
  setAccent: (accent: AccentId) => void;
  toggle: () => void;
}

export function useTheme(): ThemeApi {
  const [, setTick] = useState(0);

  useEffect(() => subscribeTheme(() => setTick((tick) => tick + 1)), []);

  const state = getThemeState();
  return {
    mode: state.mode,
    accent: state.accent,
    resolved: getResolvedTheme(),
    setMode: setThemeMode,
    setAccent: setThemeAccent,
    toggle: toggleThemeMode,
  };
}
