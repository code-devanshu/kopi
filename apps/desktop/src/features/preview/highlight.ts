// Marks the searched-for words in the preview's full text.
import type { Span } from "../../lib/ipc";

/** Matches marked past this many are left plain, to keep a long clip quick to render. */
export const MAX_MARKS = 500;

/**
 * Splits `text` into runs, marking each place any word of `query` appears, case aside.
 * Longer words win where two start at the same place.
 */
export function highlight(text: string, query: string): Span[] {
  const words = [...new Set(query.toLowerCase().split(/\s+/).filter(Boolean))].sort(
    (a, b) => b.length - a.length,
  );
  if (!words.length || !text) return text ? [{ text, matched: false }] : [];
  const escaped = words.map((w) => w.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"));
  const pattern = new RegExp(escaped.join("|"), "giu");

  const spans: Span[] = [];
  let at = 0;
  let marks = 0;
  for (const match of text.matchAll(pattern)) {
    if (marks++ === MAX_MARKS) break;
    if (match.index > at) spans.push({ text: text.slice(at, match.index), matched: false });
    spans.push({ text: match[0], matched: true });
    at = match.index + match[0].length;
  }
  if (at < text.length) spans.push({ text: text.slice(at), matched: false });
  return spans;
}
