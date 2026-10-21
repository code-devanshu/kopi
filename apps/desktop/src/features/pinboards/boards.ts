// Pinboard colors, and the order of the views the switcher shows.
import type { View } from "../history/history";
import type { Pinboard } from "../../lib/ipc";

/** The colors a pinboard can have. They read on the cream and the espresso themes. */
export const COLORS: { name: string; hex: string }[] = [
  { name: "Caramel", hex: "#D08A3A" },
  { name: "Red", hex: "#C8553D" },
  { name: "Gold", hex: "#C9A227" },
  { name: "Green", hex: "#5E8C61" },
  { name: "Teal", hex: "#3E8A8A" },
  { name: "Blue", hex: "#4A78B5" },
  { name: "Purple", hex: "#8A63A8" },
  { name: "Pink", hex: "#C25B8A" },
];

/** A new pinboard's color: the next in the palette, so neighbours differ. */
export function nextColor(boards: Pinboard[]): string {
  return COLORS[boards.length % COLORS.length].hex;
}

/** History, Pinned, Snippets, then each pinboard, as ⌘[ and ⌘] step through them. */
export function views(boards: Pinboard[]): View[] {
  return ["history", "pinned", "snippets", ...boards.map((b) => b.id)];
}

/** The view `by` steps from `view`, wrapping around. An unknown view starts from History. */
export function stepView(boards: Pinboard[], view: View, by: number): View {
  const all = views(boards);
  const at = Math.max(0, all.indexOf(view));
  return all[(at + by + all.length) % all.length];
}

/** The pinboard ids with `moved` put where `target` is. Unchanged if either is unknown. */
export function reorder(ids: number[], moved: number, target: number): number[] {
  const from = ids.indexOf(moved);
  const to = ids.indexOf(target);
  if (from < 0 || to < 0 || from === to) return ids;
  const next = ids.filter((id) => id !== moved);
  next.splice(to, 0, moved);
  return next;
}
