// A snippet's placeholders: finding them, and filling them in when it's pasted. The Rust side
// stores the body as typed and never reads them. See docs/SNIPPETS_DESIGN.md.

/** A run of a snippet's body: plain text, or a placeholder with the text it was typed as. */
export type Token =
  | { type: "text"; text: string }
  | { type: "date"; raw: string; format: string | null }
  | { type: "time"; raw: string }
  | { type: "datetime"; raw: string }
  | { type: "clipboard"; raw: string }
  | { type: "input"; raw: string; label: string }
  | { type: "cursor"; raw: string };

export type Placeholder = Exclude<Token, { type: "text" }>;

/** What filling in needs besides the time: the newest text in the history, and what was typed
 * into the form, by label. */
export type FillContext = {
  now: Date;
  clipboard: string | null;
  inputs: Readonly<Record<string, string>>;
  /** For month and day names and the system formats. The system's language when left out. */
  locale?: string;
};

export type Filled = {
  text: string;
  /** How many characters the cursor moves left after the paste: those after `{cursor}`. */
  cursorBack: number;
};

/** The label `{input}` asks for when it has none. */
export const DEFAULT_INPUT_LABEL = "Text";
/** The longest `{input:…}` label. A longer one isn't a placeholder. */
export const MAX_LABEL_CHARS = 32;

/** What the editor's Insert menu offers, with what each one inserts. */
export const INSERTABLE: { label: string; text: string }[] = [
  { label: "Date", text: "{date}" },
  { label: "Date, YYYY-MM-DD", text: "{date:YYYY-MM-DD}" },
  { label: "Time", text: "{time}" },
  { label: "Date and time", text: "{datetime}" },
  { label: "Newest clip", text: "{clipboard}" },
  { label: "Ask when pasting…", text: "{input:Name}" },
  { label: "Cursor position", text: "{cursor}" },
];

/**
 * Splits a body into text and placeholders. Unknown names and double braces stay text, so
 * code, JSON and `{{template}}` languages paste as typed. Names are case-insensitive and
 * spaces inside the braces are ignored.
 */
export function parse(body: string): Token[] {
  const tokens: Token[] = [];
  let text = "";
  let at = 0;
  while (at < body.length) {
    const open = body.indexOf("{", at);
    if (open < 0) {
      text += body.slice(at);
      break;
    }
    text += body.slice(at, open);
    // A run of two or more braces is never a placeholder, and nor is what follows it.
    let run = open;
    while (body[run] === "{") run++;
    if (run - open > 1) {
      text += body.slice(open, run);
      at = run;
      continue;
    }
    const close = body.indexOf("}", open + 1);
    const inner = close < 0 ? null : body.slice(open + 1, close);
    // A brace pair across lines is code, not a placeholder.
    const placeholder =
      inner === null || /[{\r\n]/.test(inner) ? null : read(inner, body.slice(open, close + 1));
    if (!placeholder) {
      text += "{";
      at = open + 1;
      continue;
    }
    if (text) tokens.push({ type: "text", text });
    text = "";
    tokens.push(placeholder);
    at = close + 1;
  }
  if (text) tokens.push({ type: "text", text });
  return tokens;
}

/** The placeholder `{inner}` stands for, or null if it isn't one. */
function read(inner: string, raw: string): Placeholder | null {
  const colon = inner.indexOf(":");
  const name = (colon < 0 ? inner : inner.slice(0, colon)).trim().toLowerCase();
  const arg = colon < 0 ? null : inner.slice(colon + 1).trim();
  switch (name) {
    case "date":
      return { type: "date", raw, format: arg || null };
    case "input": {
      const label = arg || DEFAULT_INPUT_LABEL;
      if ([...label].length > MAX_LABEL_CHARS) return null;
      return { type: "input", raw, label };
    }
    case "time":
    case "datetime":
    case "clipboard":
    case "cursor":
      return arg === null ? { type: name, raw } : null;
    default:
      return null;
  }
}

/** The `{input}` labels, each once, in the order they first appear. */
export function inputLabels(tokens: Token[]): string[] {
  const labels: string[] = [];
  for (const t of tokens) {
    if (t.type === "input" && !labels.includes(t.label)) labels.push(t.label);
  }
  return labels;
}

/** Whether filling in needs the newest text from the history. */
export function usesClipboard(tokens: Token[]): boolean {
  return tokens.some((t) => t.type === "clipboard");
}

/** The text to paste, and where the cursor goes. Only the first `{cursor}` counts. */
export function fill(tokens: Token[], ctx: FillContext): Filled {
  let before = "";
  let after: string | null = null;
  for (const t of tokens) {
    if (t.type === "cursor") {
      if (after === null) after = "";
      continue;
    }
    const value = valueOf(t, ctx);
    if (after === null) before += value;
    else after += value;
  }
  return { text: before + (after ?? ""), cursorBack: after === null ? 0 : graphemes(after) };
}

/** What one token pastes. */
export function valueOf(t: Token, ctx: FillContext): string {
  switch (t.type) {
    case "text":
      return t.text;
    case "date":
      return t.format ? formatDate(ctx.now, t.format, ctx.locale) : systemDate(ctx.now, { dateStyle: "full" }, ctx.locale);
    case "time":
      return systemDate(ctx.now, { timeStyle: "short" }, ctx.locale);
    case "datetime":
      return systemDate(ctx.now, { dateStyle: "medium", timeStyle: "short" }, ctx.locale);
    case "clipboard":
      return ctx.clipboard ?? "";
    case "input":
      return ctx.inputs[t.label] ?? "";
    case "cursor":
      return "";
  }
}

/** What a placeholder is called where the panel draws it as a chip. */
export function placeholderLabel(t: Placeholder): string {
  switch (t.type) {
    case "date":
      return t.format ? `Date: ${t.format}` : "Date";
    case "time":
      return "Time";
    case "datetime":
      return "Date and time";
    case "clipboard":
      return "Newest clip";
    case "input":
      return t.label;
    case "cursor":
      return "Cursor";
  }
}

/** Characters as the user sees them, so an emoji or an accented letter is one arrow press. */
export function graphemes(text: string): number {
  return Array.from(new Intl.Segmenter(undefined, { granularity: "grapheme" }).segment(text)).length;
}

function systemDate(date: Date, options: Intl.DateTimeFormatOptions, locale?: string): string {
  return new Intl.DateTimeFormat(locale, options).format(date);
}

/** The tokens `{date:…}` knows, longest first so `MMMM` isn't read as `MM` twice. Text in
 * square brackets is kept as-is. */
const DATE_TOKENS = /\[([^\]]*)\]|YYYY|YY|MMMM|MMM|MM|M|DD|D|dddd|ddd|HH|H|hh|h|mm|ss|A/g;

/** `date` in a format such as `YYYY-MM-DD` or `[Week of] D MMM`. */
export function formatDate(date: Date, format: string, locale?: string): string {
  const pad = (n: number) => String(n).padStart(2, "0");
  const name = (options: Intl.DateTimeFormatOptions) => new Intl.DateTimeFormat(locale, options).format(date);
  const hours12 = date.getHours() % 12 || 12;
  return format.replace(DATE_TOKENS, (token, literal: string | undefined) => {
    if (literal !== undefined) return literal;
    switch (token) {
      case "YYYY":
        return String(date.getFullYear());
      case "YY":
        return pad(date.getFullYear() % 100);
      case "MMMM":
        return name({ month: "long" });
      case "MMM":
        return name({ month: "short" });
      case "MM":
        return pad(date.getMonth() + 1);
      case "M":
        return String(date.getMonth() + 1);
      case "DD":
        return pad(date.getDate());
      case "D":
        return String(date.getDate());
      case "dddd":
        return name({ weekday: "long" });
      case "ddd":
        return name({ weekday: "short" });
      case "HH":
        return pad(date.getHours());
      case "H":
        return String(date.getHours());
      case "hh":
        return pad(hours12);
      case "h":
        return String(hours12);
      case "mm":
        return pad(date.getMinutes());
      case "ss":
        return pad(date.getSeconds());
      case "A":
        return date.getHours() < 12 ? "AM" : "PM";
      default:
        return token;
    }
  });
}

/** The first line of `text`, cut to `max` characters, for naming a snippet made from a clip. */
export function nameFrom(text: string, max = 64): string {
  const line = text.split(/\r?\n/).find((l) => l.trim())?.trim() ?? "";
  const chars = [...line];
  return chars.length > max ? `${chars.slice(0, max - 1).join("")}…` : line;
}
