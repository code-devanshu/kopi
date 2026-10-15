// What differs between macOS and Windows in the UI: the key shortcuts use, how shortcuts are
// written, and where Kopi's icon lives. The UI is written the macOS way, and these turn it into
// the Windows way.

/** Whether Kopi runs on Windows. WebView2 says so in its user agent. */
export const IS_WINDOWS = typeof navigator !== "undefined" && navigator.userAgent.includes("Windows");

/** The computer, as the UI calls it. */
export const DEVICE = IS_WINDOWS ? "this PC" : "this Mac";

/** Where Kopi's icon is: the menu bar on macOS, the notification area on Windows. */
export const TRAY = IS_WINDOWS ? "the notification area" : "the menu bar";

/** Whether the primary modifier is held: ⌘ on macOS, Ctrl on Windows. */
export function primary(e: Pick<KeyboardEvent, "metaKey" | "ctrlKey">, windows = IS_WINDOWS): boolean {
  return windows ? e.ctrlKey : e.metaKey;
}

const WORDS: Record<string, string> = {
  "⌘": "Ctrl",
  "⌃": "Ctrl",
  "⌥": "Alt",
  "⇧": "Shift",
  "↩": "Enter",
  "⌫": "Backspace",
  esc: "Esc",
};

/**
 * A shortcut written the macOS way, such as "⌘⌫" or "⌥↩", as this OS writes it. On Windows
 * that's "Ctrl+Backspace" and "Alt+Enter".
 */
export function keys(mac: string, windows = IS_WINDOWS): string {
  if (!windows) return mac;
  const parts: string[] = [];
  let rest = mac;
  while (rest.length > 1 && "⌘⌃⌥⇧".includes(rest[0])) {
    parts.push(WORDS[rest[0]]);
    rest = rest.slice(1);
  }
  parts.push(WORDS[rest] ?? rest);
  return parts.join("+");
}
