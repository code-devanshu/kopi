// What the list shows: the search box, the filters, the results and the selected row. It lives
// outside React so it can be tested without a DOM. Components read it with useHistory().
import type { ClipKind, SearchHit, SearchPage, SearchQuery, Snippet } from "../../lib/ipc";

/** Results asked for at a time. */
export const PAGE = 50;
/** The most the backend returns in one page. */
export const MAX_PAGE = 200;
/** How long to wait after a clip event before searching again, so a burst runs once. */
export const REFRESH_DELAY_MS = 50;

/** What the list shows: the whole history, the pinned clips, the snippets, or a pinboard by
 * id. */
export type View = "history" | "pinned" | "snippets" | number;

export type HistoryState = {
  view: View;
  text: string;
  kind: ClipKind | null;
  appId: string | null;
  /** Rows above the clips: every matching snippet in the Snippets view, and in History the
   * snippets whose keyword is exactly the search. */
  snippets: Snippet[];
  hits: SearchHit[];
  hasMore: boolean;
  /** False until the first results arrive. */
  loaded: boolean;
  /** Why the last search failed, until one succeeds. */
  error: string | null;
  /** The selected row, counting the snippets first, or -1 when there are no results. */
  selected: number;
};

export type Search = (query: SearchQuery) => Promise<SearchPage>;

/** Searching snippets: every match, or only an exact keyword. */
export type SnippetSearch = {
  all: (text: string) => Promise<Snippet[]>;
  keyword: (text: string) => Promise<Snippet[]>;
};

const NO_SNIPPETS: SnippetSearch = {
  all: () => Promise.resolve([]),
  keyword: () => Promise.resolve([]),
};

const INITIAL: HistoryState = {
  view: "history",
  text: "",
  kind: null,
  appId: null,
  snippets: [],
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
  #snippets: SnippetSearch;
  /** Bumped by every search, so a slower, older answer is dropped. */
  #seq = 0;
  #loadingMore = false;
  #refreshTimer: ReturnType<typeof setTimeout> | undefined;

  constructor(search: Search, snippets: SnippetSearch = NO_SNIPPETS) {
    this.#search = search;
    this.#snippets = snippets;
  }

  getSnapshot = (): HistoryState => this.#state;

  subscribe = (listener: () => void): (() => void) => {
    this.#listeners.add(listener);
    return () => this.#listeners.delete(listener);
  };

  get selectedHit(): SearchHit | undefined {
    return rowAt(this.#state, this.#state.selected)?.hit;
  }

  get selectedSnippet(): Snippet | undefined {
    return rowAt(this.#state, this.#state.selected)?.snippet;
  }

  setView(view: View) {
    if (view === this.#state.view) return;
    this.#set({ view });
    void this.#load(false);
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

  /** Clears the search box and starts from the top. The view and filters stay. */
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
    const last = rowCount(this.#state) - 1;
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
      const [page, snippets] = await Promise.all([
        before.view === "snippets"
          ? Promise.resolve<SearchPage>({ hits: [], hasMore: false })
          : this.#search({ ...this.#query(), offset: 0, limit }),
        this.#findSnippets(before),
      ]);
      if (seq !== this.#seq) return;
      const now = this.#state;
      const next = { snippets, hits: page.hits };
      let selected = 0;
      if (keep) {
        // The same row if it's still there, otherwise the row in its place.
        const was = rowAt(now, now.selected);
        const found = was ? indexOf(next, was) : -1;
        selected = found >= 0 ? found : Math.min(now.selected, rowCount(next) - 1);
      }
      this.#set({
        ...next,
        hasMore: page.hasMore,
        loaded: true,
        error: null,
        selected: rowCount(next) ? Math.max(selected, 0) : -1,
      });
    } catch (e) {
      if (seq === this.#seq) this.#set({ loaded: true, error: String(e) });
    }
  }

  /** The snippet rows for `state`. A failed keyword lookup only leaves them out of History. */
  #findSnippets({ view, text, kind, appId }: HistoryState): Promise<Snippet[]> {
    if (view === "snippets") return this.#snippets.all(text);
    if (view !== "history" || !text.trim() || kind !== null || appId !== null) return Promise.resolve([]);
    return this.#snippets.keyword(text).catch(() => []);
  }

  #query(): SearchQuery {
    const { view, text, kind, appId } = this.#state;
    return {
      text,
      kind,
      sourceAppId: appId,
      pinned: view === "pinned",
      pinboardId: typeof view === "number" ? view : null,
    };
  }

  #set(patch: Partial<HistoryState>) {
    this.#state = { ...this.#state, ...patch };
    for (const listener of this.#listeners) listener();
  }
}

/** A row of the list: a snippet or a clip. */
export type Row = { snippet: Snippet; hit?: undefined } | { hit: SearchHit; snippet?: undefined };

type Rows = Pick<HistoryState, "snippets" | "hits">;

export function rowCount(rows: Rows): number {
  return rows.snippets.length + rows.hits.length;
}

/** The row at `index`, snippets first. */
export function rowAt(rows: Rows, index: number): Row | undefined {
  if (index < 0) return undefined;
  if (index < rows.snippets.length) return { snippet: rows.snippets[index] };
  const hit = rows.hits[index - rows.snippets.length];
  return hit && { hit };
}

/** The element id of a row, for aria-activedescendant. */
export function rowId(row: Row): string {
  return row.snippet ? `snippet-${row.snippet.id}` : `clip-${row.hit.id}`;
}

function indexOf(rows: Rows, row: Row): number {
  if (row.snippet) return rows.snippets.findIndex((s) => s.id === row.snippet.id);
  const at = rows.hits.findIndex((h) => h.id === row.hit.id);
  return at < 0 ? -1 : rows.snippets.length + at;
}
