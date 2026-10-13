import { ClipboardList, LayoutGrid, Pin, SearchX, TriangleAlert } from "lucide-react";
import { KIND_LABELS } from "../../lib/format";
import type { ClipKind } from "../../lib/ipc";

/** Placeholder rows shaped like the real ones, until the first results arrive. */
export function LoadingRows() {
  return (
    <div className="list" aria-busy="true" aria-label="Loading clipboard history">
      {Array.from({ length: 7 }, (_, i) => (
        <div key={i} className="row skeleton" style={{ position: "relative" }} aria-hidden="true">
          <span className="leading" />
          <div className="row-body">
            <span className="bar" style={{ width: `${70 - ((i * 17) % 35)}%` }} />
            <span className="bar short" />
          </div>
        </div>
      ))}
    </div>
  );
}

type EmptyProps = {
  /** "pinned", a pinboard's name, or null for the whole history. */
  place: "pinned" | { board: string } | null;
  text: string;
  kind: ClipKind | null;
  appName: string | null;
  onClearFilters: () => void;
};

/** No results: nothing copied yet, or nothing that matches. */
export function Empty({ place, text, kind, appName, onClearFilters }: EmptyProps) {
  const filtered = kind !== null || appName !== null;
  if (!text.trim() && !filtered && place === "pinned") {
    return (
      <div className="state" role="status">
        <Pin size={28} strokeWidth={1.5} aria-hidden="true" />
        <h2>No pinned clips</h2>
        <p>Press ⌘P on a clip to keep it here. Pinned clips are never cleaned up.</p>
      </div>
    );
  }
  if (!text.trim() && !filtered && typeof place === "object" && place !== null) {
    return (
      <div className="state" role="status">
        <LayoutGrid size={28} strokeWidth={1.5} aria-hidden="true" />
        <h2>{place.board} is empty</h2>
        <p>Drag clips onto its tab, or add them from a clip’s menu with ⌘K.</p>
      </div>
    );
  }
  if (!text.trim() && !filtered) {
    return (
      <div className="state" role="status">
        <ClipboardList size={28} strokeWidth={1.5} aria-hidden="true" />
        <h2>Nothing copied yet</h2>
        <p>Copy something in any app and it shows up here.</p>
      </div>
    );
  }
  const scope = [kind && KIND_LABELS[kind].toLowerCase(), appName && `from ${appName}`]
    .filter(Boolean)
    .join(" ");
  return (
    <div className="state" role="status">
      <SearchX size={28} strokeWidth={1.5} aria-hidden="true" />
      <h2>{text.trim() ? <>No matches for “{text.trim()}”</> : "No clips here"}</h2>
      {filtered ? (
        <p>
          Only showing {scope} clips.{" "}
          <button type="button" className="link" onMouseDown={(e) => e.preventDefault()} onClick={onClearFilters}>
            Show all
          </button>
        </p>
      ) : (
        <p>Try fewer or shorter words.</p>
      )}
    </div>
  );
}

export function LoadError({ message, onRetry }: { message: string; onRetry: () => void }) {
  return (
    <div className="state" role="alert">
      <TriangleAlert size={28} strokeWidth={1.5} aria-hidden="true" />
      <h2>Couldn’t load your history</h2>
      <p className="detail">{message}</p>
      <button type="button" className="button" onMouseDown={(e) => e.preventDefault()} onClick={onRetry}>
        Try again
      </button>
    </div>
  );
}
