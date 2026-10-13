import { expect, test } from "vitest";
import { highlight, MAX_MARKS } from "./highlight";

/** Spans as text, with matches in brackets. */
function show(text: string, query: string): string {
  return highlight(text, query)
    .map((s) => (s.matched ? `[${s.text}]` : s.text))
    .join("");
}

test.each([
  ["no query", "", "no query"],
  ["", "a", ""],
  ["Hello world", "world", "Hello [world]"],
  ["Hello World", "WORLD hello", "[Hello] [World]"],
  ["abab", "ab", "[ab][ab]"],
  ["xab", "a ab", "x[ab]"],
  ["cost: $5 (approx.)", "$5 (approx.)", "cost: [$5] [(approx.)]"],
  ["a.b axb", ".", "a[.]b axb"],
  ["東京タワー", "タワー", "東京[タワー]"],
  ["line one\nline two", "line", "[line] one\n[line] two"],
  ["  spaced  ", "   ", "  spaced  "],
])("highlight(%j, %j)", (text, query, want) => {
  expect(show(text, query)).toBe(want);
});

test("stops marking after the limit", () => {
  const spans = highlight("a ".repeat(MAX_MARKS + 10), "a");
  expect(spans.filter((s) => s.matched)).toHaveLength(MAX_MARKS);
  expect(spans.map((s) => s.text).join("")).toBe("a ".repeat(MAX_MARKS + 10));
});
