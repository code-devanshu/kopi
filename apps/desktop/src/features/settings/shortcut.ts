// Shortcuts as the global-shortcut plugin writes them ("shift+super+KeyV"): recording one from
// a key press, and showing one as macOS does (⇧⌘V).

/** The default, as `hotkeys::DEFAULT` on macOS. */
export const DEFAULT_SHORTCUT = "shift+super+KeyV";

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

/** Whether the plugin can register `code` as a shortcut's key. */
function isKey(code: string): boolean {
  return /^(Key[A-Z]|Digit[0-9]|F([1-9]|1[0-9]|2[0-4]))$/.test(code) || code in NAMED;
}

/** What a key press in the recorder makes. */
export function record(e: RecordKey): Recorded {
  const mods = MODIFIERS.filter(([flag]) => e[flag]).map(([, name]) => name);
  if (!isKey(e.code)) return { type: "partial", held: format(mods.join("+")) };
  if (!e.metaKey && !e.ctrlKey && !e.altKey) {
    return { type: "invalid", message: "Include ⌘, ⌃ or ⌥ in the shortcut." };
  }
  return { type: "done", shortcut: [...mods, e.code].join("+") };
}

/** The symbols for each part, in macOS order: ["⇧", "⌘", "V"]. Empty for an empty string. */
export function parts(shortcut: string): string[] {
  const tokens = shortcut.split("+").filter(Boolean);
  const mods = new Set(tokens.map((t) => t.toLowerCase()));
  const symbols = SYMBOLS.filter(([name]) => mods.has(name)).map(([, symbol]) => symbol);
  const key = tokens.find((t) => !SYMBOLS.some(([name]) => name === t.toLowerCase()));
  return key ? [...symbols, keyLabel(key)] : symbols;
}

/** A shortcut as macOS shows it: ⇧⌘V. */
export function format(shortcut: string): string {
  return parts(shortcut).join("");
}

function keyLabel(code: string): string {
  if (code.startsWith("Key")) return code.slice(3);
  if (code.startsWith("Digit")) return code.slice(5);
  return NAMED[code] ?? code;
}
