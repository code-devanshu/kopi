// What the list shows: the search box, the filters, the results and the selected row. It lives
// outside React so it can be tested without a DOM. Components read it with useHistory().
import type { ClipKind, SearchHit, SearchPage, SearchQuery } from "../../lib/ipc";

/** Results asked for at a time. */
export const PAGE = 50;
/** The most the backend returns in one page. */
export const MAX_PAGE = 200;
/** How long to wait after a clip event before searching again, so a burst runs once. */
export const REFRESH_DELAY_MS = 50;

export type HistoryState = {
  text: string;
  kind: ClipKind | null;
  appId: string | null;
  hits: SearchHit[];
  hasMore: boolean;
  /** False until the first results arrive. */
  loaded: boolean;
  /** Why the last search failed, until one succeeds. */
  error: string | null;
  /** The selected row, or -1 when there are no results. */
  selected: number;
};

export type Search = (query: SearchQuery) => Promise<SearchPage>;

const INITIAL: HistoryState = {
  text: "",
  kind: null,
  appId: null,
  hits: [],
  hasMore: false,
  loaded: false,
  error: null,
  selected: -1,
};

export class History {
  #state = INITIAL;
  #listeners = new Set<() => void>();
  #search: Search;
  /** Bumped by every search, so a slower, older answer is dropped. */
  #seq = 0;
  #loadingMore = false;
  #refreshTimer: ReturnType<typeof setTimeout> | undefined;

  constructor(search: Search) {
    this.#search = search;
  }

  getSnapshot = (): HistoryState => this.#state;

  subscribe = (listener: () => void): (() => void) => {
    this.#listeners.add(listener);
    return () => this.#listeners.delete(listener);
  };

  get selectedHit(): SearchHit | undefined {
    return this.#state.hits[this.#state.selected];
  }

  setText(text: string) {
    if (text === this.#state.text) return;
    this.#set({ text });
    void this.#load(false);
  }

  setKind(kind: ClipKind | null) {
    if (kind === this.#state.kind) return;
    this.#set({ kind });
    void this.#load(false);
  }

  setApp(appId: string | null) {
    if (appId === this.#state.appId) return;
    this.#set({ appId });
    void this.#load(false);
  }

  /** Clears the search box and starts from the top. Filters stay. */
  reset() {
    this.#set({ text: "" });
    return this.#load(false);
  }

  /** Searches again, keeping the selected clip and as many rows as are loaded. */
  refresh() {
    clearTimeout(this.#refreshTimer);
    this.#refreshTimer = undefined;
    return this.#load(true);
  }

  /** Calls refresh() after REFRESH_DELAY_MS, once for a burst of calls. */
  refreshSoon() {
    if (this.#refreshTimer !== undefined) return;
    this.#refreshTimer = setTimeout(() => void this.refresh(), REFRESH_DELAY_MS);
  }

  /** Loads the next page, if there is one and none is loading. */
  async loadMore() {
    const { hits, hasMore } = this.#state;
    if (!hasMore || this.#loadingMore) return;
    this.#loadingMore = true;
    const seq = this.#seq;
    try {
      const page = await this.#search({ ...this.#query(), offset: hits.length, limit: PAGE });
      if (seq !== this.#seq) return;
      // Clips can move between pages while the user scrolls. Keep the first copy.
      const seen = new Set(hits.map((h) => h.id));
      this.#set({
        hits: [...hits, ...page.hits.filter((h) => !seen.has(h.id))],
        hasMore: page.hasMore,
      });
    } catch (e) {
      if (seq === this.#seq) this.#set({ error: String(e) });
    } finally {
      this.#loadingMore = false;
    }
  }

  select(index: number) {
    const last = this.#state.hits.length - 1;
    if (last < 0) return;
    this.#set({ selected: Math.max(0, Math.min(last, index)) });
  }

  move(by: number) {
    this.select(this.#state.selected + by);
  }

  async #load(keep: boolean) {
    const seq = ++this.#seq;
    const before = this.#state;
    const limit = keep ? Math.min(Math.max(before.hits.length, PAGE), MAX_PAGE) : PAGE;
    try {
      const page = await this.#search({ ...this.#query(), offset: 0, limit });
      if (seq !== this.#seq) return;
      const now = this.#state;
      let selected = 0;
      if (keep) {
        // The same clip if it's still there, otherwise the row in its place.
        const id = now.hits[now.selected]?.id;
        const found = page.hits.findIndex((h) => h.id === id);
        selected = found >= 0 ? found : Math.min(now.selected, page.hits.length - 1);
      }
      this.#set({
        hits: page.hits,
        hasMore: page.hasMore,
        loaded: true,
        error: null,
        selected: page.hits.length ? Math.max(selected, 0) : -1,
      });
    } catch (e) {
      if (seq === this.#seq) this.#set({ loaded: true, error: String(e) });
    }
  }

  #query(): SearchQuery {
    const { text, kind, appId } = this.#state;
    return { text, kind, sourceAppId: appId };
  }

  #set(patch: Partial<HistoryState>) {
    this.#state = { ...this.#state, ...patch };
    for (const listener of this.#listeners) listener();
  }
}
