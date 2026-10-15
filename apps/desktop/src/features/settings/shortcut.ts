// Shortcuts as the global-shortcut plugin writes them ("shift+super+KeyV"): recording one from
// a key press, and showing one as macOS does (⇧⌘V) or as Windows does (Ctrl+Shift+V).

import { IS_WINDOWS } from "../../lib/platform";

/** The default, as `hotkeys::DEFAULT`. */
export const DEFAULT_SHORTCUT = IS_WINDOWS ? "shift+control+KeyV" : "shift+super+KeyV";

/** The parts of a KeyboardEvent the recorder looks at. */
export type RecordKey = Pick<KeyboardEvent, "code" | "metaKey" | "ctrlKey" | "altKey" | "shiftKey">;

export type Recorded =
  /** Only modifiers so far, or a key the recorder ignores. Shows what's held. */
  | { type: "partial"; held: string }
  /** A shortcut to save. */
  | { type: "done"; shortcut: string }
  /** A key without ⌘, ⌃ or ⌥, which would fire on ordinary typing. */
  | { type: "invalid"; message: string };

// The plugin's order, so a recorded shortcut matches what it writes back.
const MODIFIERS = [
  ["shiftKey", "shift"],
  ["ctrlKey", "control"],
  ["altKey", "alt"],
  ["metaKey", "super"],
] as const;

// macOS's order for showing them.
const SYMBOLS: [string, string][] = [
  ["control", "⌃"],
  ["alt", "⌥"],
  ["shift", "⇧"],
  ["super", "⌘"],
];

// Windows's order and names.
const WINDOWS_NAMES: [string, string][] = [
  ["control", "Ctrl"],
  ["alt", "Alt"],
  ["shift", "Shift"],
  ["super", "Win"],
];

const NAMED: Record<string, string> = {
  Space: "Space",
  Enter: "↩",
  Tab: "⇥",
  Backspace: "⌫",
  Delete: "⌦",
  Home: "↖",
  End: "↘",
  PageUp: "⇞",
  PageDown: "⇟",
  ArrowUp: "↑",
  ArrowDown: "↓",
  ArrowLeft: "←",
  ArrowRight: "→",
  Backquote: "`",
  Minus: "-",
  Equal: "=",
  BracketLeft: "[",
  BracketRight: "]",
  Backslash: "\\",
  Semicolon: ";",
  Quote: "'",
  Comma: ",",
  Period: ".",
  Slash: "/",
};

/** Keys Windows names in words where macOS has a symbol. */
const WINDOWS_KEYS: Record<string, string> = {
  Enter: "Enter",
  Tab: "Tab",
  Backspace: "Backspace",
  Delete: "Delete",
  Home: "Home",
  End: "End",
  PageUp: "PgUp",
  PageDown: "PgDn",
};

/** Whether the plugin can register `code` as a shortcut's key. */
function isKey(code: string): boolean {
  return /^(Key[A-Z]|Digit[0-9]|F([1-9]|1[0-9]|2[0-4]))$/.test(code) || code in NAMED;
}

/** What a key press in the recorder makes. */
export function record(e: RecordKey, windows = IS_WINDOWS): Recorded {
  const mods = MODIFIERS.filter(([flag]) => e[flag]).map(([, name]) => name);
  if (!isKey(e.code)) return { type: "partial", held: format(mods.join("+"), windows) };
  if (!e.metaKey && !e.ctrlKey && !e.altKey) {
    const needed = windows ? "Ctrl, Alt or Win" : "⌘, ⌃ or ⌥";
    return { type: "invalid", message: `Include ${needed} in the shortcut.` };
  }
  return { type: "done", shortcut: [...mods, e.code].join("+") };
}

/**
 * Each part in the OS's order, as the OS names it: ["⇧", "⌘", "V"] on macOS, ["Ctrl", "Shift",
 * "V"] on Windows. Empty for an empty string.
 */
export function parts(shortcut: string, windows = IS_WINDOWS): string[] {
  const names = windows ? WINDOWS_NAMES : SYMBOLS;
  const tokens = shortcut.split("+").filter(Boolean);
  const mods = new Set(tokens.map((t) => t.toLowerCase()));
  const symbols = names.filter(([name]) => mods.has(name)).map(([, symbol]) => symbol);
  const key = tokens.find((t) => !names.some(([name]) => name === t.toLowerCase()));
  return key ? [...symbols, keyLabel(key, windows)] : symbols;
}

/** A shortcut as the OS shows it: ⇧⌘V on macOS, Ctrl+Shift+V on Windows. */
export function format(shortcut: string, windows = IS_WINDOWS): string {
  return parts(shortcut, windows).join(windows ? "+" : "");
}

function keyLabel(code: string, windows: boolean): string {
  if (code.startsWith("Key")) return code.slice(3);
  if (code.startsWith("Digit")) return code.slice(5);
  return (windows ? WINDOWS_KEYS[code] : undefined) ?? NAMED[code] ?? code;
}
