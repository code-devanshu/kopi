// The choices for each history limit. A limit is a number or null for unlimited.

import { size } from "../../lib/format";

export type Choice = { value: number | null; label: string };

const GB = 1024 ** 3;
const MB = 1024 ** 2;

const count = new Intl.NumberFormat();

export const ITEM_CHOICES: Choice[] = [
  ...[100, 500, 1000, 5000, 10_000, 50_000].map((n) => ({ value: n, label: itemLabel(n) })),
  { value: null, label: "Unlimited" },
];

export const AGE_CHOICES: Choice[] = [
  { value: 1, label: "1 day" },
  { value: 7, label: "1 week" },
  { value: 30, label: "1 month" },
  { value: 90, label: "3 months" },
  { value: 365, label: "1 year" },
  { value: null, label: "Never" },
];

export const SIZE_CHOICES: Choice[] = [
  { value: 256 * MB, label: "256 MB" },
  { value: 512 * MB, label: "512 MB" },
  { value: GB, label: "1 GB" },
  { value: 2 * GB, label: "2 GB" },
  { value: 5 * GB, label: "5 GB" },
  { value: 10 * GB, label: "10 GB" },
  { value: null, label: "Unlimited" },
];

/** `choices`, plus `current` if it isn't one of them, such as a value set by an older Kopi. */
export function withCurrent(choices: Choice[], current: number | null, label: (n: number) => string): Choice[] {
  if (choices.some((c) => c.value === current) || current === null) return choices;
  return [...choices, { value: current, label: label(current) }].sort(
    (a, b) => (a.value ?? Infinity) - (b.value ?? Infinity),
  );
}

export function itemLabel(n: number): string {
  return `${count.format(n)} clips`;
}

export function ageLabel(days: number): string {
  return days === 1 ? "1 day" : `${days} days`;
}

export function sizeLabel(bytes: number): string {
  return size(bytes);
}

/** A choice's value for a `<select>`, where null can't be a value. */
export function toOption(value: number | null): string {
  return value === null ? "none" : String(value);
}

export function fromOption(option: string): number | null {
  return option === "none" ? null : Number(option);
}
