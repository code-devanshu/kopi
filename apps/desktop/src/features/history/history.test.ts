import { afterEach, describe, expect, test, vi } from "vitest";
import type { SearchHit, SearchPage, SearchQuery } from "../../lib/ipc";
import { History, MAX_PAGE, PAGE, REFRESH_DELAY_MS } from "./history";

function hit(id: number): SearchHit {
  return {
    id,
    kind: "text",
    contentHash: String(id).padStart(64, "0"),
    sourceAppName: "Notes",
    sourceAppId: "com.apple.Notes",
    lastUsedAt: id,
    pinned: false,
    metadata: null,
    snippet: [{ text: `clip ${id}`, matched: false }],
  };
}

/** A backend holding clips `ids`, newest first, that answers each search when told to. */
class Backend {
  ids: number[];
  calls: SearchQuery[] = [];
  #pending: (() => void)[] = [];
  fail: string | null = null;

  constructor(ids: number[]) {
    this.ids = ids;
  }

  search = (query: SearchQuery): Promise<SearchPage> => {
    this.calls.push(query);
    return new Promise((resolve, reject) => {
      this.#pending.push(() => {
        if (this.fail) return reject(this.fail);
        const offset = query.offset ?? 0;
        const limit = query.limit ?? PAGE;
        const ids = this.ids.slice(offset, offset + limit);
        resolve({ hits: ids.map(hit), hasMore: offset + limit < this.ids.length });
      });
    });
  };

  /** Answers the waiting searches, oldest first, or the one at `index`. */
  async answer(index?: number) {
    const pending =
      index === undefined ? this.#pending.splice(0) : this.#pending.splice(index, 1);
    pending.forEach((answer) => answer());
    await new Promise((r) => setTimeout(r, 0));
  }
}

function setup(ids: number[]) {
  const backend = new Backend(ids);
  const history = new History(backend.search);
  return { backend, history, state: () => history.getSnapshot() };
}

/** The query fields for the whole history, with no filters. */
const ALL = { kind: null, sourceAppId: null, pinned: false, pinboardId: null };

const range = (from: number, to: number) =>
  Array.from({ length: to - from + 1 }, (_, i) => to - i);

afterEach(() => {
  vi.useRealTimers();
});

test("starts empty and loads the first page", async () => {
  const { backend, history, state } = setup(range(1, 120));
  expect(state()).toMatchObject({ loaded: false, hits: [], selected: -1 });

  const done = history.reset();
  await backend.answer();
  await done;
  expect(backend.calls).toEqual([
    { ...ALL, text: "", offset: 0, limit: PAGE },
  ]);
  expect(state()).toMatchObject({ loaded: true, hasMore: true, selected: 0, error: null });
  expect(state().hits.map((h) => h.id).slice(0, 3)).toEqual([120, 119, 118]);
  expect(history.selectedHit?.id).toBe(120);
});

test("typing and filters search from the top, and only the latest answer counts", async () => {
  const { backend, history, state } = setup(range(1, 10));
  history.reset();
  await backend.answer();
  history.select(4);

  history.setText("a");
  history.setText("ab");
  history.setKind("link");
  history.setApp("com.apple.Safari");
  // The same value again doesn't search.
  history.setApp("com.apple.Safari");
  expect(backend.calls.slice(1)).toEqual([
    { ...ALL, text: "a", offset: 0, limit: PAGE },
    { ...ALL, text: "ab", offset: 0, limit: PAGE },
    { ...ALL, text: "ab", kind: "link", offset: 0, limit: PAGE },
    { ...ALL, text: "ab", kind: "link", sourceAppId: "com.apple.Safari", offset: 0, limit: PAGE },
  ]);

  // The last search answers first. The older ones arrive later and are dropped.
  backend.ids = [7, 3];
  await backend.answer(3);
  expect(state().hits.map((h) => h.id)).toEqual([7, 3]);
  expect(state().selected).toBe(0);
  backend.ids = [99];
  await backend.answer();
  expect(state().hits.map((h) => h.id)).toEqual([7, 3]);

  // Reset clears the text but keeps the filters.
  history.reset();
  expect(backend.calls.at(-1)).toMatchObject({ text: "", kind: "link" });
});

test("the view picks the pinned clips or a pinboard, and keeps the search", async () => {
  const { backend, history, state } = setup(range(1, 10));
  history.reset();
  await backend.answer();
  history.select(3);
  history.setText("deploy");

  history.setView("pinned");
  history.setView(7);
  history.setView(7);
  history.setView("history");
  expect(backend.calls.slice(2)).toEqual([
    { ...ALL, text: "deploy", pinned: true, offset: 0, limit: PAGE },
    { ...ALL, text: "deploy", pinboardId: 7, offset: 0, limit: PAGE },
    { ...ALL, text: "deploy", offset: 0, limit: PAGE },
  ]);
  await backend.answer();
  expect(state()).toMatchObject({ view: "history", selected: 0 });

  // Reset keeps the view.
  history.setView(7);
  history.reset();
  expect(backend.calls.at(-1)).toMatchObject({ text: "", pinboardId: 7 });
});

test("pages load once at a time and skip clips already shown", async () => {
  const { backend, history, state } = setup(range(1, 120));
  history.reset();
  await backend.answer();

  history.loadMore();
  history.loadMore();
  expect(backend.calls).toHaveLength(2);
  expect(backend.calls[1]).toMatchObject({ offset: PAGE, limit: PAGE });
  // A clip moved to the top meanwhile, so the next page repeats one.
  backend.ids = [70, ...range(1, 120).filter((id) => id !== 70)];
  await backend.answer();
  expect(state().hits).toHaveLength(2 * PAGE - 1);
  expect(new Set(state().hits.map((h) => h.id)).size).toBe(state().hits.length);

  history.loadMore();
  await backend.answer();
  expect(state().hasMore).toBe(false);
  history.loadMore();
  expect(backend.calls).toHaveLength(3);
});

test("a page that arrives after a new search is dropped", async () => {
  const { backend, history, state } = setup(range(1, 120));
  history.reset();
  await backend.answer();
  history.loadMore();
  history.setText("x");
  backend.ids = [5];
  await backend.answer(1);
  await backend.answer();
  expect(state().hits.map((h) => h.id)).toEqual([5]);
});

describe("refresh", () => {
  test("keeps the selected clip and the loaded rows, up to a page's limit", async () => {
    const { backend, history, state } = setup(range(1, 300));
    history.reset();
    await backend.answer();
    history.loadMore();
    await backend.answer();
    history.select(60);
    const selected = history.selectedHit!.id;

    // Two clips arrive at the top.
    backend.ids = [302, 301, ...backend.ids];
    history.refresh();
    expect(backend.calls.at(-1)).toMatchObject({ offset: 0, limit: 2 * PAGE });
    await backend.answer();
    expect(history.selectedHit?.id).toBe(selected);
    expect(state().selected).toBe(62);

    for (let i = 0; i < 4; i++) {
      history.loadMore();
      await backend.answer();
    }
    history.refresh();
    expect(backend.calls.at(-1)).toMatchObject({ limit: MAX_PAGE });
  });

  test("selects the row in its place when the selected clip is gone", async () => {
    const { backend, history, state } = setup(range(1, 10));
    history.reset();
    await backend.answer();
    history.select(9);
    backend.ids = backend.ids.filter((id) => id !== 1 && id !== 2);
    history.refresh();
    await backend.answer();
    expect(state().selected).toBe(7);

    backend.ids = [];
    history.refresh();
    await backend.answer();
    expect(state()).toMatchObject({ hits: [], selected: -1 });
  });

  test("runs once for a burst of events", async () => {
    vi.useFakeTimers();
    const { backend, history } = setup([1]);
    history.refreshSoon();
    history.refreshSoon();
    history.refreshSoon();
    expect(backend.calls).toHaveLength(0);
    vi.advanceTimersByTime(REFRESH_DELAY_MS);
    expect(backend.calls).toHaveLength(1);
    history.refreshSoon();
    vi.advanceTimersByTime(REFRESH_DELAY_MS);
    expect(backend.calls).toHaveLength(2);
  });
});

test("errors are shown until a search succeeds", async () => {
  const { backend, history, state } = setup([1, 2]);
  backend.fail = "database is locked";
  history.reset();
  await backend.answer();
  expect(state()).toMatchObject({ loaded: true, error: "database is locked" });

  backend.fail = null;
  history.refresh();
  await backend.answer();
  expect(state()).toMatchObject({ error: null, selected: 0 });
});

test("selection stays within the results", async () => {
  const { backend, history, state } = setup([3, 2, 1]);
  history.move(1);
  expect(state().selected).toBe(-1);
  history.reset();
  await backend.answer();
  history.move(-1);
  expect(state().selected).toBe(0);
  history.move(10);
  expect(state().selected).toBe(2);
  history.select(1);
  expect(history.selectedHit?.id).toBe(2);
});

test("subscribers hear every change until they unsubscribe", async () => {
  const { backend, history } = setup([1]);
  const heard = vi.fn();
  const stop = history.subscribe(heard);
  history.setText("a");
  await backend.answer();
  expect(heard).toHaveBeenCalledTimes(2);
  stop();
  history.setText("b");
  expect(heard).toHaveBeenCalledTimes(2);
});
