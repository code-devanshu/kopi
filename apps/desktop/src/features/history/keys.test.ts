import { expect, test } from "vitest";
import { actionFor, type Action, type Key } from "./keys";

/** A key press from a short name: "cmd+alt+Enter", "Space", "cmd+Digit3". */
function key(combo: string): Key {
  const parts = combo.split("+");
  const name = parts.pop()!;
  const mods = new Set(parts);
  const codes: Record<string, [string, string]> = {
    Space: [" ", "Space"],
    Digit1: ["1", "Digit1"],
    Digit3: ["3", "Digit3"],
    Digit9: ["9", "Digit9"],
    Digit0: ["0", "Digit0"],
    Y: ["y", "KeyY"],
    N: ["n", "KeyN"],
    P: ["p", "KeyP"],
    A: ["a", "KeyA"],
    C: ["c", "KeyC"],
  };
  const [k, code] = codes[name] ?? [name, name];
  return {
    key: k,
    code,
    metaKey: mods.has("cmd"),
    ctrlKey: mods.has("ctrl"),
    altKey: mods.has("alt"),
    shiftKey: mods.has("shift"),
    isComposing: false,
    keyCode: 0,
  };
}

const cases: [string, boolean, Action | null][] = [
  ["ArrowDown", false, { type: "move", by: 1 }],
  ["ArrowUp", true, { type: "move", by: -1 }],
  ["shift+ArrowDown", false, null],
  ["cmd+ArrowDown", false, { type: "last" }],
  ["cmd+ArrowUp", true, { type: "first" }],
  ["ctrl+N", false, { type: "move", by: 1 }],
  ["ctrl+P", true, { type: "move", by: -1 }],
  ["PageDown", false, { type: "page", by: 1 }],
  ["PageUp", false, { type: "page", by: -1 }],
  ["Enter", true, { type: "paste", plain: false }],
  ["alt+Enter", true, { type: "paste", plain: true }],
  ["shift+Enter", false, null],
  ["cmd+Enter", false, null],
  ["cmd+Digit1", false, { type: "quickPaste", index: 0, plain: false }],
  ["cmd+Digit9", true, { type: "quickPaste", index: 8, plain: false }],
  ["cmd+alt+Digit3", false, { type: "quickPaste", index: 2, plain: true }],
  ["cmd+Digit0", false, null],
  ["Digit1", false, null],
  ["cmd+shift+Digit1", false, null],
  ["cmd+Backspace", true, { type: "delete" }],
  ["Backspace", true, null],
  ["alt+Backspace", true, null],
  ["Escape", true, { type: "hide" }],
  ["Space", false, { type: "togglePreview" }],
  ["Space", true, null],
  ["shift+Space", false, null],
  ["cmd+Y", true, { type: "togglePreview" }],
  ["Y", false, null],
  ["A", false, null],
  ["cmd+A", true, null],
  ["cmd+C", true, { type: "copy", plain: false }],
  ["cmd+alt+C", false, { type: "copy", plain: true }],
  ["cmd+shift+C", false, null],
  ["C", false, null],
];

test.each(cases)("%s (typing: %s)", (combo, typing, want) => {
  expect(actionFor(key(combo), typing)).toEqual(want);
});

test("keys are left alone while an input method composes", () => {
  expect(actionFor({ ...key("Enter"), isComposing: true }, true)).toBeNull();
  // The press-and-hold accent picker reports only keyCode 229.
  expect(actionFor({ ...key("ArrowDown"), keyCode: 229 }, true)).toBeNull();
  expect(actionFor({ ...key("Escape"), keyCode: 229 }, true)).toBeNull();
});
