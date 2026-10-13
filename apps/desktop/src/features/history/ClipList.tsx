import { useEffect, useLayoutEffect, useRef, useState } from "react";
import type { SearchHit } from "../../lib/ipc";
import { ClipRow, ROW_HEIGHT } from "./ClipRow";

/** Rows rendered above and below the visible ones, so fast scrolling doesn't show gaps. */
const OVERSCAN = 6;
/** Loading the next page starts this many rows before the end. */
const LOAD_AHEAD = 15;

type Props = {
  id: string;
  /** Changes with the search text and filters. */
  searchKey: string;
  hits: SearchHit[];
  selected: number;
  now: number;
  thumbnailAttempts: ReadonlyMap<string, number>;
  onSelect: (index: number) => void;
  onPick: (index: number) => void;
  onNearEnd: () => void;
  /** How many rows fit, for Page Up and Page Down. */
  onPageSize: (rows: number) => void;
};

/** Only the rows in view are in the DOM, so a long history stays quick. */
export function ClipList({
  id,
  searchKey,
  hits,
  selected,
  now,
  thumbnailAttempts,
  onSelect,
  onPick,
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

  const start = Math.max(0, Math.floor(scrollTop / ROW_HEIGHT) - OVERSCAN);
  const end = Math.min(hits.length, Math.ceil((scrollTop + height) / ROW_HEIGHT) + OVERSCAN);
  const nearEnd = end + LOAD_AHEAD >= hits.length || selected + LOAD_AHEAD >= hits.length;
  useEffect(() => {
    if (nearEnd) onNearEnd();
  }, [nearEnd, hits.length, onNearEnd]);

  return (
    <div
      ref={ref}
      id={id}
      className="list"
      role="listbox"
      aria-label="Clipboard history"
      onScroll={(e) => setScrollTop(e.currentTarget.scrollTop)}
    >
      <div className="list-inner" style={{ height: hits.length * ROW_HEIGHT }}>
        {hits.slice(start, end).map((hit, i) => (
          <ClipRow
            key={hit.id}
            hit={hit}
            index={start + i}
            selected={start + i === selected}
            now={now}
            thumbnailAttempt={thumbnailAttempts.get(hit.contentHash) ?? 0}
            onSelect={onSelect}
            onPick={onPick}
          />
        ))}
      </div>
    </div>
  );
}
