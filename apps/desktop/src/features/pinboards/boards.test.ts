import { expect, test } from "vitest";
import type { Pinboard } from "../../lib/ipc";
import { COLORS, nextColor, reorder, stepView, views } from "./boards";

const board = (id: number): Pinboard => ({ id, name: `Board ${id}`, color: null, clipCount: 0 });
const boards = [board(4), board(2), board(9)];

test("views run from History through Pinned to each pinboard", () => {
  expect(views(boards)).toEqual(["history", "pinned", 4, 2, 9]);
  expect(views([])).toEqual(["history", "pinned"]);
});

test("stepping wraps around", () => {
  expect(stepView(boards, "history", 1)).toBe("pinned");
  expect(stepView(boards, "pinned", 1)).toBe(4);
  expect(stepView(boards, 9, 1)).toBe("history");
  expect(stepView(boards, "history", -1)).toBe(9);
  expect(stepView(boards, 2, -1)).toBe(4);
  // A pinboard that was deleted counts as History.
  expect(stepView(boards, 77, 1)).toBe("pinned");
  expect(stepView([], "pinned", 1)).toBe("history");
});

test("a moved pinboard takes the target's place", () => {
  expect(reorder([1, 2, 3, 4], 1, 3)).toEqual([2, 3, 1, 4]);
  expect(reorder([1, 2, 3, 4], 4, 2)).toEqual([1, 4, 2, 3]);
  expect(reorder([1, 2, 3, 4], 2, 2)).toEqual([1, 2, 3, 4]);
  expect(reorder([1, 2, 3, 4], 9, 2)).toEqual([1, 2, 3, 4]);
  expect(reorder([1, 2, 3, 4], 2, 9)).toEqual([1, 2, 3, 4]);
});

test("new pinboards cycle through the palette", () => {
  expect(nextColor([])).toBe(COLORS[0].hex);
  expect(nextColor(boards)).toBe(COLORS[3].hex);
  expect(nextColor(Array.from({ length: COLORS.length }, (_, i) => board(i)))).toBe(COLORS[0].hex);
  for (const { hex } of COLORS) expect(hex).toMatch(/^#[0-9A-F]{6}$/);
});
