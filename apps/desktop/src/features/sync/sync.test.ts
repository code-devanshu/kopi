import { describe, expect, it } from "vitest";
import type { SyncDevice } from "../../lib/ipc";
import { cleanCode, deviceStatus, formatCode, since, timeLeft } from "./sync";

const NOW = Date.UTC(2026, 9, 20, 12, 0, 0);

const device = (patch: Partial<SyncDevice> = {}): SyncDevice => ({
  id: "d",
  name: "Windows PC",
  os: "windows",
  connected: false,
  needsUpdate: false,
  lastSeenAt: null,
  stale: false,
  ...patch,
});

describe("codes", () => {
  it("are shown in two groups of three", () => {
    expect(formatCode("482913")).toBe("482 913");
    expect(formatCode("4829")).toBe("4829");
  });

  it("keep only digits as they're typed", () => {
    expect(cleanCode("482 913")).toBe("482913");
    expect(cleanCode("48-29-13-77")).toBe("482913");
    expect(cleanCode("abc")).toBe("");
  });

  it("count down", () => {
    expect(timeLeft(NOW + 245_000, NOW)).toBe("4:05");
    expect(timeLeft(NOW - 1, NOW)).toBe("0:00");
  });
});

describe("a computer's status", () => {
  it("says how long ago it synced", () => {
    expect(since(NOW - 10_000, NOW)).toBe("just now");
    expect(since(NOW - 60_000, NOW)).toBe("1 minute ago");
    expect(since(NOW - 3 * 3_600_000, NOW)).toBe("3 hours ago");
    expect(since(NOW - 2 * 86_400_000, NOW)).toBe("2 days ago");
    expect(since(NOW - 40 * 86_400_000, NOW, "en-GB")).toMatch(/^on \d+ \w+ 2026$/);
  });

  it("puts the most useful thing first", () => {
    expect(deviceStatus(device(), true, NOW)).toBe("Not synced yet");
    expect(deviceStatus(device({ connected: true, lastSeenAt: NOW }), true, NOW)).toBe("Connected");
    expect(deviceStatus(device({ needsUpdate: true, connected: true }), true, NOW)).toMatch(/newer Kopi/);
    expect(deviceStatus(device({ stale: true, lastSeenAt: NOW - 100 * 86_400_000 }), true, NOW)).toMatch(
      /90 days/,
    );
    expect(deviceStatus(device({ lastSeenAt: NOW - 3_600_000 }), true, NOW)).toBe("Last synced 1 hour ago");
    // While sync is off, only when it last synced.
    expect(deviceStatus(device({ connected: true, lastSeenAt: NOW - 60_000 }), false, NOW)).toBe(
      "Last synced 1 minute ago",
    );
  });
});
