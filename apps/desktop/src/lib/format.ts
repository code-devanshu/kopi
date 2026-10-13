// How times, sizes, kinds and colors read in the panel.
import type { ClipKind } from "./ipc";

const MINUTE = 60_000;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;

/** A short age for a list row: "now", "5m", "3h", "2d", then the date. */
export function ago(at: number, now: number, locale?: string): string {
  const age = Math.max(0, now - at);
  if (age < MINUTE) return "now";
  if (age < HOUR) return `${Math.floor(age / MINUTE)}m`;
  if (age < DAY) return `${Math.floor(age / HOUR)}h`;
  if (age < 7 * DAY) return `${Math.floor(age / DAY)}d`;
  const date = new Date(at);
  const sameYear = date.getFullYear() === new Date(now).getFullYear();
  return date.toLocaleDateString(locale, {
    day: "numeric",
    month: "short",
    year: sameYear ? undefined : "numeric",
  });
}

/** A full date and time, for the preview. */
export function dateTime(at: number, locale?: string): string {
  return new Date(at).toLocaleString(locale, { dateStyle: "medium", timeStyle: "short" });
}

/** Bytes as Finder shows them, in powers of 1000. */
export function size(bytes: number): string {
  if (bytes < 1000) return `${bytes} B`;
  const units = ["KB", "MB", "GB"];
  let value = bytes / 1000;
  let unit = 0;
  while (value >= 1000 && unit < units.length - 1) {
    value /= 1000;
    unit++;
  }
  return `${value < 10 ? value.toFixed(1) : Math.round(value)} ${units[unit]}`;
}

export const KIND_LABELS: Record<ClipKind, string> = {
  text: "Text",
  rich_text: "Rich text",
  link: "Link",
  image: "Image",
  file: "File",
  color: "Color",
  code: "Code",
};

/** Every kind, in the order the filter lists them. */
export const KINDS = Object.keys(KIND_LABELS) as ClipKind[];

/** `#RRGGBB`, or `#RRGGBBAA` when it isn't opaque. Null for anything but 3 or 4 bytes. */
export function hex(rgba: number[] | null | undefined): string | null {
  if (!rgba || rgba.length < 3 || rgba.length > 4) return null;
  if (rgba.some((v) => !Number.isInteger(v) || v < 0 || v > 255)) return null;
  const [r, g, b, a = 255] = rgba;
  const parts = a === 255 ? [r, g, b] : [r, g, b, a];
  return `#${parts.map((v) => v.toString(16).padStart(2, "0")).join("").toUpperCase()}`;
}
