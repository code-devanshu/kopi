import { useEffect } from "react";
import { commands, events, unwrap, type Theme } from "../../lib/ipc";

/** Sets `data-theme` on <html>, which picks the colors in global.css. "system" follows the OS. */
export function applyTheme(theme: Theme) {
  const root = document.documentElement;
  if (theme === "system") delete root.dataset.theme;
  else root.dataset.theme = theme;
}

/** Applies the theme setting now, and again whenever it changes in Settings. */
export function useThemeSetting() {
  useEffect(() => {
    unwrap(commands.getSettings()).then((s) => applyTheme(s.theme), () => {});
    const unlisten = events.themeChanged.listen(({ payload }) => applyTheme(payload.theme));
    return () => void unlisten.then((stop) => stop());
  }, []);
}
