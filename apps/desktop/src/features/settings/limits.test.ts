import { expect, test } from "vitest";
import { AGE_CHOICES, ageLabel, fromOption, ITEM_CHOICES, itemLabel, toOption, withCurrent } from "./limits";

test("keeps a value that isn't a choice, in order", () => {
  const choices = withCurrent(AGE_CHOICES, 14, ageLabel);
  expect(choices.map((c) => c.value)).toEqual([1, 7, 14, 30, 90, 365, null]);
  expect(choices[2].label).toBe("14 days");
});

test("adds nothing for a choice or unlimited", () => {
  expect(withCurrent(ITEM_CHOICES, 500, itemLabel)).toBe(ITEM_CHOICES);
  expect(withCurrent(ITEM_CHOICES, null, itemLabel)).toBe(ITEM_CHOICES);
});

test("round-trips through a select's value", () => {
  for (const value of [null, 0, 7, 2 * 1024 ** 3]) {
    expect(fromOption(toOption(value))).toBe(value);
  }
});
