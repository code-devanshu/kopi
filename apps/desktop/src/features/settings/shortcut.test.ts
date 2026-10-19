import { expect, test } from "vitest";
import { clash, DEFAULT_SHORTCUT, format, parts, record, type RecordKey } from "./shortcut";

function press(code: string, mods: string[] = []): RecordKey {
  return {
    code,
    metaKey: mods.includes("cmd"),
    ctrlKey: mods.includes("ctrl"),
    altKey: mods.includes("alt"),
    shiftKey: mods.includes("shift"),
  };
}

test("records in the plugin's order", () => {
  expect(record(press("KeyV", ["cmd", "shift"]))).toEqual({ type: "done", shortcut: DEFAULT_SHORTCUT });
  expect(record(press("Space", ["alt", "ctrl"]))).toEqual({ type: "done", shortcut: "control+alt+Space" });
  expect(record(press("F5", ["cmd"]))).toEqual({ type: "done", shortcut: "super+F5" });
  expect(record(press("Comma", ["cmd", "alt"]))).toEqual({ type: "done", shortcut: "alt+super+Comma" });
});

test("shows the modifiers held so far", () => {
  expect(record(press("MetaLeft", ["cmd"]))).toEqual({ type: "partial", held: "⌘" });
  expect(record(press("ShiftLeft", ["cmd", "shift"]))).toEqual({ type: "partial", held: "⇧⌘" });
  expect(record(press("Escape", []))).toEqual({ type: "partial", held: "" });
});

test("needs a modifier other than shift", () => {
  expect(record(press("KeyV")).type).toBe("invalid");
  expect(record(press("KeyV", ["shift"])).type).toBe("invalid");
  expect(record(press("F5")).type).toBe("invalid");
});

test("formats as macOS does", () => {
  expect(format(DEFAULT_SHORTCUT)).toBe("⇧⌘V");
  expect(format("control+alt+Space")).toBe("⌃⌥Space");
  expect(format("super+Digit1")).toBe("⌘1");
  expect(format("alt+super+Comma")).toBe("⌥⌘,");
  expect(format("Shift+Super+ArrowUp")).toBe("⇧⌘↑");
  expect(parts("shift+control+KeyK")).toEqual(["⌃", "⇧", "K"]);
  expect(parts("")).toEqual([]);
});

test("formats as Windows does", () => {
  expect(format("shift+control+KeyV", true)).toBe("Ctrl+Shift+V");
  expect(format("alt+super+Comma", true)).toBe("Alt+Win+,");
  expect(format("control+PageDown", true)).toBe("Ctrl+PgDn");
  expect(record(press("ControlLeft", ["ctrl"]), true)).toEqual({ type: "partial", held: "Ctrl" });
  expect(record(press("KeyV", ["shift"]), true)).toEqual({
    type: "invalid",
    message: "Include Ctrl, Alt or Win in the shortcut.",
  });
});

test("warns about Ctrl+Shift+V only on Windows", () => {
  expect(clash("shift+control+KeyV", true)).toMatch(/paste without formatting/);
  expect(clash("Control+Shift+KeyV", true)).not.toBeNull();
  expect(clash("alt+shift+KeyV", true)).toBeNull();
  expect(clash("shift+control+KeyV", false)).toBeNull();
  expect(clash("shift+super+KeyV", false)).toBeNull();
});
