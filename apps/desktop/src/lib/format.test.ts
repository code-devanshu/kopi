import { describe, expect, test } from "vitest";
import { ago, hex, size } from "./format";

const NOW = Date.UTC(2026, 9, 13, 12, 0, 0);
const MINUTE = 60_000;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;

describe("ago", () => {
  test.each([
    [0, "now"],
    [59_999, "now"],
    [MINUTE, "1m"],
    [59 * MINUTE, "59m"],
    [HOUR, "1h"],
    [23 * HOUR + 59 * MINUTE, "23h"],
    [DAY, "1d"],
    [6 * DAY, "6d"],
    // Clocks can disagree by a little.
    [-5 * MINUTE, "now"],
  ])("%i ms ago is %s", (age, want) => {
    expect(ago(NOW - age, NOW, "en-GB")).toBe(want);
  });

  test("a week or more shows the date, with the year when it differs", () => {
    expect(ago(NOW - 7 * DAY, NOW, "en-GB")).toBe("6 Oct");
    expect(ago(Date.UTC(2025, 11, 30, 12), NOW, "en-GB")).toBe("30 Dec 2025");
  });
});

test.each([
  [0, "0 B"],
  [999, "999 B"],
  [1000, "1.0 KB"],
  [1234, "1.2 KB"],
  [64_000, "64 KB"],
  [999_499, "999 KB"],
  [1_500_000, "1.5 MB"],
  [50_000_000, "50 MB"],
  [3_200_000_000, "3.2 GB"],
  [4_000_000_000_000, "4000 GB"],
])("size(%i) is %s", (bytes, want) => {
  expect(size(bytes)).toBe(want);
});

test.each([
  [[30, 144, 255, 255], "#1E90FF"],
  [[30, 144, 255], "#1E90FF"],
  [[0, 0, 0, 128], "#00000080"],
  [[255, 255, 255, 0], "#FFFFFF00"],
  [[1, 2], null],
  [[1, 2, 3, 4, 5], null],
  [[256, 0, 0], null],
  [[-1, 0, 0], null],
  [[0.5, 0, 0], null],
  [null, null],
  [undefined, null],
])("hex(%j) is %s", (rgba, want) => {
  expect(hex(rgba)).toBe(want);
});
