// How Settings → Sync words a computer's state and shows codes.
import type { ClipKind, SyncDevice } from "../../lib/ipc";

const MINUTE = 60_000;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;

/** The kinds that can sync, in the order Settings lists them. Files never do. */
export const SYNC_KINDS: ClipKind[] = ["text", "rich_text", "link", "code", "color", "image"];

/** A code as it's shown and read out: `482 913`. */
export function formatCode(code: string): string {
  return code.length === 6 ? `${code.slice(0, 3)} ${code.slice(3)}` : code;
}

/** What's typed into the code field, kept to its digits, at most 6. */
export function cleanCode(typed: string): string {
  return typed.replace(/\D/g, "").slice(0, 6);
}

/** "just now", "5 minutes ago", "3 hours ago", "2 days ago", then the date. */
export function since(at: number, now: number, locale?: string): string {
  const age = Math.max(0, now - at);
  const plural = (n: number, unit: string) => `${n} ${unit}${n === 1 ? "" : "s"} ago`;
  if (age < MINUTE) return "just now";
  if (age < HOUR) return plural(Math.floor(age / MINUTE), "minute");
  if (age < DAY) return plural(Math.floor(age / HOUR), "hour");
  if (age < 30 * DAY) return plural(Math.floor(age / DAY), "day");
  return `on ${new Date(at).toLocaleDateString(locale, { day: "numeric", month: "short", year: "numeric" })}`;
}

/** The line under a computer's name. */
export function deviceStatus(device: SyncDevice, syncOn: boolean, now: number): string {
  const last = device.lastSeenAt === null ? "Not synced yet" : `Last synced ${since(device.lastSeenAt, now)}`;
  if (!syncOn) return last;
  if (device.needsUpdate) return "Has a newer Kopi. Update this one to sync with it.";
  if (device.connected) return "Connected";
  if (device.stale) return "Not seen for 90 days. Remove it and pair it again, so deleted clips stay deleted.";
  return last;
}

/** Minutes and seconds left on a code: `4:05`. */
export function timeLeft(expiresAt: number, now: number): string {
  const seconds = Math.max(0, Math.ceil((expiresAt - now) / 1000));
  return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, "0")}`;
}
