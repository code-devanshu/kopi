import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { COPIED_CLOSE_MS, COPY_CLOSE_MS, Paster, type Notice } from "./paster";

let notices: (Notice | null)[];
let hidden: number;
let openedAccessibility: number;

function paster(outcome: "pasted" | "copied" | Error = "pasted") {
  const calls: string[] = [];
  const p = new Paster({
    paste: async (id, plain) => {
      calls.push(`paste ${id} ${plain}`);
      if (outcome instanceof Error) throw outcome.message;
      return outcome;
    },
    copy: async (id, plain) => {
      calls.push(`copy ${id} ${plain}`);
    },
    hide: () => hidden++,
    notify: (n) => notices.push(n),
    openAccessibility: () => openedAccessibility++,
  });
  return { p, calls };
}

const last = () => notices[notices.length - 1];

beforeEach(() => {
  vi.useFakeTimers();
  notices = [];
  hidden = 0;
  openedAccessibility = 0;
});

afterEach(() => vi.useRealTimers());

test("a paste shows nothing: the panel is already hidden", async () => {
  const { p, calls } = paster("pasted");
  await p.paste(3, true);
  expect(calls).toEqual(["paste 3 true"]);
  expect(last()).toBeNull();
  vi.runAllTimers();
  expect(hidden).toBe(0);
});

test("without Accessibility it says Copied, then closes", async () => {
  const { p } = paster("copied");
  await p.paste(3, false);
  expect(last()?.message).toBe("Copied. Press ⌘V to paste.");
  expect(p.closing).toBe(true);
  vi.advanceTimersByTime(COPIED_CLOSE_MS - 1);
  expect(hidden).toBe(0);
  vi.advanceTimersByTime(1);
  expect(hidden).toBe(1);
  expect(last()).toBeNull();
  expect(p.closing).toBe(false);
});

test("its button closes the panel and opens the Accessibility settings", async () => {
  const { p } = paster("copied");
  await p.paste(3, false);
  last()?.action?.run();
  expect(hidden).toBe(1);
  expect(openedAccessibility).toBe(1);
  expect(p.closing).toBe(false);
  vi.runAllTimers();
  expect(hidden).toBe(1);
});

test("⌘V while it says Copied closes the panel at once", async () => {
  const { p } = paster("copied");
  await p.paste(3, false);
  p.closeNow();
  expect(hidden).toBe(1);
  vi.runAllTimers();
  expect(hidden).toBe(1);
});

test("copy confirms, then closes sooner", async () => {
  const { p, calls } = paster();
  await p.copy(5, true);
  expect(calls).toEqual(["copy 5 true"]);
  expect(last()?.message).toBe("Copied.");
  vi.advanceTimersByTime(COPY_CLOSE_MS);
  expect(hidden).toBe(1);
});

test("a failure stays up and leaves the panel open", async () => {
  const { p } = paster(new Error("no clip 3"));
  await p.paste(3, false);
  expect(last()).toEqual({ message: "Couldn't paste.", detail: "no clip 3", tone: "error" });
  vi.runAllTimers();
  expect(hidden).toBe(0);
});

test("a second Return while a paste is on its way is ignored", async () => {
  const { p, calls } = paster("pasted");
  const first = p.paste(1, false);
  await p.paste(2, false);
  await first;
  expect(calls).toEqual(["paste 1 false"]);
});

test("reset drops the message and the planned close", async () => {
  const { p } = paster("copied");
  await p.paste(3, false);
  p.reset();
  vi.runAllTimers();
  expect(hidden).toBe(0);
  expect(last()).toBeNull();
});
