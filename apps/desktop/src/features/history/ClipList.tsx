import { useEffect, useLayoutEffect, useRef, useState, type PointerEvent } from "react";
import type { SearchHit, Snippet } from "../../lib/ipc";
import { SnippetRow } from "../snippets/SnippetRow";
import { ClipRow, ROW_HEIGHT } from "./ClipRow";

/** Rows rendered above and below the visible ones, so fast scrolling doesn't show gaps. */
const OVERSCAN = 6;
/** Loading the next page starts this many rows before the end. */
const LOAD_AHEAD = 15;

type Props = {
  id: string;
  /** Changes with the search text and filters. */
  searchKey: string;
  /** Rows above the clips. */
  snippets: Snippet[];
  hits: SearchHit[];
  query: string;
  /** Whether snippet rows say they're snippets, as in History. */
  labelSnippets: boolean;
  /** The snippet a dragged snippet would land on. */
  dropSnippet: number | null;
  selected: number;
  now: number;
  thumbnailAttempts: ReadonlyMap<string, number>;
  onSelect: (index: number) => void;
  onPick: (index: number) => void;
  onMenu: (index: number, x: number, y: number) => void;
  onDragStart: (index: number, e: PointerEvent) => void;
  onNearEnd: () => void;
  /** How many rows fit, for Page Up and Page Down. */
  onPageSize: (rows: number) => void;
};

/** Only the rows in view are in the DOM, so a long history stays quick. */
export function ClipList({
  id,
  searchKey,
  snippets,
  hits,
  query,
  labelSnippets,
  dropSnippet,
  selected,
  now,
  thumbnailAttempts,
  onSelect,
  onPick,
  onMenu,
  onDragStart,
  onNearEnd,
  onPageSize,
}: Props) {
  const ref = useRef<HTMLDivElement>(null);
  const [scrollTop, setScrollTop] = useState(0);
  const [height, setHeight] = useState(0);

  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    const observer = new ResizeObserver(() => {
      setHeight(el.clientHeight);
      onPageSize(Math.max(1, Math.floor(el.clientHeight / ROW_HEIGHT) - 1));
    });
    observer.observe(el);
    return () => observer.disconnect();
  }, [onPageSize]);

  // Keep the selected row in view as the keyboard moves it.
  useLayoutEffect(() => {
    const el = ref.current;
    if (!el || selected < 0) return;
    const top = selected * ROW_HEIGHT;
    if (top < el.scrollTop) el.scrollTop = top;
    else if (top + ROW_HEIGHT > el.scrollTop + el.clientHeight) {
      el.scrollTop = top + ROW_HEIGHT - el.clientHeight;
    }
  }, [selected]);

  // A new search starts at the top.
  useLayoutEffect(() => {
    if (ref.current) ref.current.scrollTop = 0;
  }, [searchKey]);

  const count = snippets.length + hits.length;
  const start = Math.max(0, Math.floor(scrollTop / ROW_HEIGHT) - OVERSCAN);
  const end = Math.min(count, Math.ceil((scrollTop + height) / ROW_HEIGHT) + OVERSCAN);
  const nearEnd = end + LOAD_AHEAD >= count || selected + LOAD_AHEAD >= count;
  useEffect(() => {
    if (nearEnd) onNearEnd();
  }, [nearEnd, count, onNearEnd]);

  const rows = [];
  for (let index = start; index < end; index++) {
    if (index < snippets.length) {
      const snippet = snippets[index];
      rows.push(
        <SnippetRow
          key={`s${snippet.id}`}
          snippet={snippet}
          index={index}
          selected={index === selected}
          query={query}
          labelled={labelSnippets}
          dropping={dropSnippet === snippet.id}
          onSelect={onSelect}
          onPick={onPick}
          onMenu={onMenu}
          onDragStart={onDragStart}
        />,
      );
      continue;
    }
    const hit = hits[index - snippets.length];
    rows.push(
      <ClipRow
        key={hit.id}
        hit={hit}
        index={index}
        selected={index === selected}
        now={now}
        thumbnailAttempt={thumbnailAttempts.get(hit.contentHash) ?? 0}
        onSelect={onSelect}
        onPick={onPick}
        onMenu={onMenu}
        onDragStart={onDragStart}
      />,
    );
  }

  return (
    <div
      ref={ref}
      id={id}
      className="list"
      role="listbox"
      aria-label={hits.length || !snippets.length ? "Clipboard history" : "Snippets"}
      onScroll={(e) => setScrollTop(e.currentTarget.scrollTop)}
    >
      <div className="list-inner" style={{ height: count * ROW_HEIGHT }}>
        {rows}
      </div>
    </div>
  );
}
