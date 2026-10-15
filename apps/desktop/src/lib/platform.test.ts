import { expect, test } from "vitest";
import { keys, primary } from "./platform";

test("writes shortcuts the Windows way on Windows", () => {
  expect(keys("⌘⌫", true)).toBe("Ctrl+Backspace");
  expect(keys("⌥↩", true)).toBe("Alt+Enter");
  expect(keys("⌘1–9", true)).toBe("Ctrl+1–9");
  expect(keys("⌘Y", true)).toBe("Ctrl+Y");
  expect(keys("↩", true)).toBe("Enter");
  expect(keys("esc", true)).toBe("Esc");
  expect(keys("Space", true)).toBe("Space");
});

test("leaves them alone on macOS", () => {
  expect(keys("⌘⌫", false)).toBe("⌘⌫");
  expect(keys("esc", false)).toBe("esc");
});

test("reads the primary modifier", () => {
  expect(primary({ metaKey: true, ctrlKey: false }, false)).toBe(true);
  expect(primary({ metaKey: true, ctrlKey: false }, true)).toBe(false);
  expect(primary({ metaKey: false, ctrlKey: true }, true)).toBe(true);
});
