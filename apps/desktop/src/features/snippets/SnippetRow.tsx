import { TextQuote } from "lucide-react";
import { memo, type PointerEvent } from "react";
import type { Snippet } from "../../lib/ipc";
import { keys } from "../../lib/platform";
import { ROW_HEIGHT } from "../history/ClipRow";
import { highlight } from "../preview/highlight";

type Props = {
  snippet: Snippet;
  index: number;
  selected: boolean;
  /** The search, to mark in the name. */
  query: string;
  /** In History, where it's there for its keyword, it says it's a snippet. */
  labelled: boolean;
  /** Something being dragged would land here. */
  dropping: boolean;
  onSelect: (index: number) => void;
  onPick: (index: number) => void;
  onMenu: (index: number, x: number, y: number) => void;
  onDragStart: (index: number, e: PointerEvent) => void;
};

/** A snippet in the list: its name, its first line, and its keyword. */
export const SnippetRow = memo(function SnippetRow({
  snippet,
  index,
  selected,
  query,
  labelled,
  dropping,
  onSelect,
  onPick,
  onMenu,
  onDragStart,
}: Props) {
  const first = firstLine(snippet.body);
  const meta = [labelled ? "Snippet" : null, first].filter(Boolean).join(" · ");
  return (
    <div
      id={`snippet-${snippet.id}`}
      role="option"
      aria-selected={selected}
      className={dropping ? "row dropping" : "row"}
      style={{ top: index * ROW_HEIGHT }}
      data-drop={`snippet:${snippet.id}`}
      onMouseDown={(e) => e.preventDefault()}
      onClick={() => onSelect(index)}
      onDoubleClick={() => onPick(index)}
      onPointerDown={(e) => onDragStart(index, e)}
      onContextMenu={(e) => {
        e.preventDefault();
        onMenu(index, e.clientX, e.clientY);
      }}
    >
      <span className="leading icon" title="Snippet">
        <TextQuote size={16} strokeWidth={1.75} aria-hidden="true" />
      </span>
      <div className="row-body">
        <div className="row-text">
          {highlight(snippet.name, query).map((span, i) =>
            span.matched ? <mark key={i}>{span.text}</mark> : <span key={i}>{span.text}</span>,
          )}
        </div>
        <div className="row-meta">{meta || <span className="placeholder">Empty</span>}</div>
      </div>
      {snippet.keyword && <span className="row-keyword">{snippet.keyword}</span>}
      {index < 9 && (
        <kbd className="row-key" aria-hidden="true">
          {keys(`⌘${index + 1}`)}
        </kbd>
      )}
    </div>
  );
});

/** The first line with text, for the row. */
function firstLine(body: string): string {
  return body.split(/\r?\n/).find((l) => l.trim())?.trim().slice(0, 200) ?? "";
}
