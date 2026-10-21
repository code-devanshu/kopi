import { describe, expect, test } from "vitest";
import { fill, formatDate, graphemes, inputLabels, nameFrom, parse, usesClipboard, type FillContext } from "./template";

// Tuesday 21 October 2026, 18:45:09 local time.
const NOW = new Date(2026, 9, 21, 18, 45, 9);
const CTX: FillContext = { now: NOW, clipboard: "https://track.example.com/123", inputs: {}, locale: "en-GB" };

function paste(body: string, ctx: Partial<FillContext> = {}) {
  return fill(parse(body), { ...CTX, ...ctx });
}

describe("parse", () => {
  test("finds each placeholder", () => {
    expect(parse("Hi {input:Name}, on {date} at {time}{cursor}.")).toEqual([
      { type: "text", text: "Hi " },
      { type: "input", raw: "{input:Name}", label: "Name" },
      { type: "text", text: ", on " },
      { type: "date", raw: "{date}", format: null },
      { type: "text", text: " at " },
      { type: "time", raw: "{time}" },
      { type: "cursor", raw: "{cursor}" },
      { type: "text", text: "." },
    ]);
  });

  test("names are case-insensitive and spaces inside are ignored", () => {
    expect(parse("{ Date }{CLIPBOARD}{ input : First name }")).toEqual([
      { type: "date", raw: "{ Date }", format: null },
      { type: "clipboard", raw: "{CLIPBOARD}" },
      { type: "input", raw: "{ input : First name }", label: "First name" },
    ]);
  });

  test("unknown names, JSON and code stay as typed", () => {
    for (const body of [
      "{name}",
      '{"a": 1, "date": 2}',
      "function f() {\n  return cursor;\n}",
      "a { color: red }",
      "{time:HH}",
      "{",
      "}{",
      "{date",
      "{{date}",
    ]) {
      expect(paste(body)).toEqual({ text: body, cursorBack: 0 });
    }
  });

  test("double braces are never placeholders", () => {
    expect(paste("{{date}} and {{{cursor}}} and {{ input:Name }}")).toEqual({
      text: "{{date}} and {{{cursor}}} and {{ input:Name }}",
      cursorBack: 0,
    });
  });

  test("a brace before a placeholder stays", () => {
    expect(paste("{x {date:YYYY}")).toEqual({ text: "{x 2026", cursorBack: 0 });
  });

  test("a label is at most 32 characters", () => {
    const long = "L".repeat(33);
    expect(paste(`{input:${long}}`).text).toBe(`{input:${long}}`);
    expect(inputLabels(parse(`{input:${"L".repeat(32)}}`))).toEqual(["L".repeat(32)]);
  });
});

describe("fill", () => {
  test("dates and times in the system's formats", () => {
    // The exact punctuation depends on the ICU version.
    const system = (options: Intl.DateTimeFormatOptions) => new Intl.DateTimeFormat("en-GB", options).format(NOW);
    expect(paste("{date}").text).toBe(system({ dateStyle: "full" }));
    expect(paste("{date}").text).toMatch(/^Wednesday,? 21 October 2026$/);
    expect(paste("{time}").text).toBe("18:45");
    expect(paste("{datetime}").text).toBe(system({ dateStyle: "medium", timeStyle: "short" }));
  });

  test("the clipboard is the newest text in the history, or nothing", () => {
    expect(paste("Track: {clipboard}").text).toBe("Track: https://track.example.com/123");
    expect(paste("Track: {clipboard}", { clipboard: null }).text).toBe("Track: ");
    expect(usesClipboard(parse("{clipboard}"))).toBe(true);
    expect(usesClipboard(parse("{{clipboard}}"))).toBe(false);
  });

  test("inputs are asked once per label, in order", () => {
    const tokens = parse("{input:Name} {input} {input:Order} {input:Name} {input:}");
    expect(inputLabels(tokens)).toEqual(["Name", "Text", "Order"]);
    expect(
      fill(tokens, { ...CTX, inputs: { Name: "Rahul", Text: "hi", Order: "#42" } }).text,
    ).toBe("Rahul hi #42 Rahul hi");
    expect(fill(tokens, CTX).text).toBe("    ");
  });

  test("the cursor goes back by what follows the first {cursor}", () => {
    expect(paste("<b>{cursor}</b>")).toEqual({ text: "<b></b>", cursorBack: 4 });
    expect(paste("{cursor}abc{cursor}de")).toEqual({ text: "abcde", cursorBack: 5 });
    expect(paste("end{cursor}")).toEqual({ text: "end", cursorBack: 0 });
    expect(paste("no cursor")).toEqual({ text: "no cursor", cursorBack: 0 });
    // Placeholders after it count as what they paste.
    expect(paste("{cursor} {date:YYYY}")).toEqual({ text: " 2026", cursorBack: 5 });
  });

  test("characters are counted as the user sees them", () => {
    expect(graphemes("👍🏽")).toBe(1);
    expect(graphemes("👨‍👩‍👧")).toBe(1);
    expect(graphemes("é")).toBe(1);
    expect(graphemes("a\nb")).toBe(3);
    expect(graphemes("a\r\nb")).toBe(3);
    expect(paste("{cursor}🇮🇳 é\n!")).toEqual({ text: "🇮🇳 é\n!", cursorBack: 5 });
  });

  test("a 64 KiB body fills in", () => {
    const body = "x".repeat(64 * 1024 - 20) + "{cursor}{date:YYYY}";
    const filled = paste(body);
    expect(filled.text.length).toBe(64 * 1024 - 20 + 4);
    expect(filled.cursorBack).toBe(4);
  });
});

describe("formatDate", () => {
  test("each token", () => {
    const f = (format: string) => formatDate(NOW, format, "en-GB");
    expect(f("YYYY-MM-DD")).toBe("2026-10-21");
    expect(f("YY M D")).toBe("26 10 21");
    expect(f("MMMM MMM")).toBe("October Oct");
    expect(f("dddd ddd")).toBe("Wednesday Wed");
    expect(f("HH:mm:ss")).toBe("18:45:09");
    expect(f("h hh A")).toBe("6 06 PM");
    expect(formatDate(new Date(2026, 0, 5, 0, 7), "H:mm h A, DD/MM", "en-GB")).toBe("0:07 12 AM, 05/01");
  });

  test("bracketed text and other characters are kept", () => {
    expect(formatDate(NOW, "[Week of] D MMM, [YYYY] YYYY!", "en-GB")).toBe("Week of 21 Oct, YYYY 2026!");
  });

  test("names follow the language", () => {
    expect(formatDate(NOW, "dddd D MMMM", "fr-FR")).toBe("mercredi 21 octobre");
  });
});

test("nameFrom takes the first line with text", () => {
  expect(nameFrom("\n  Thanks for your order  \nMore")).toBe("Thanks for your order");
  expect(nameFrom("a".repeat(70))).toBe(`${"a".repeat(63)}…`);
  expect(nameFrom(" \n ")).toBe("");
});
