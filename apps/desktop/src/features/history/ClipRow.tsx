import { Pin } from "lucide-react";
import { memo, useState, type PointerEvent } from "react";
import { KindIcon } from "../../components/KindIcon";
import { thumbnailUrl } from "../../lib/assets";
import { ago, hex, KIND_LABELS } from "../../lib/format";
import type { SearchHit } from "../../lib/ipc";
import { keys } from "../../lib/platform";

/** Every row has this height, so the list can work out which rows are in view. */
export const ROW_HEIGHT = 52;

type Props = {
  hit: SearchHit;
  index: number;
  selected: boolean;
  now: number;
  /** Bumped when this clip's thumbnail is written, to load it again. */
  thumbnailAttempt: number;
  onSelect: (index: number) => void;
  onPick: (index: number) => void;
  /** Right-click: the clip's menu, at the pointer. */
  onMenu: (index: number, x: number, y: number) => void;
  /** A press that may become a drag onto a pinboard. */
  onDragStart: (index: number, e: PointerEvent) => void;
};

export const ClipRow = memo(function ClipRow({
  hit,
  index,
  selected,
  now,
  thumbnailAttempt,
  onSelect,
  onPick,
  onMenu,
  onDragStart,
}: Props) {
  const meta = [detail(hit), hit.sourceAppName, ago(hit.lastUsedAt, now)].filter(Boolean);
  return (
    <div
      id={`clip-${hit.id}`}
      role="option"
      aria-selected={selected}
      className="row"
      style={{ top: index * ROW_HEIGHT }}
      // The search box keeps focus, so typing still searches after a click.
      onMouseDown={(e) => e.preventDefault()}
      onClick={() => onSelect(index)}
      onDoubleClick={() => onPick(index)}
      onPointerDown={(e) => onDragStart(index, e)}
      onContextMenu={(e) => {
        e.preventDefault();
        onMenu(index, e.clientX, e.clientY);
      }}
    >
      <Leading hit={hit} attempt={thumbnailAttempt} />
      <div className="row-body">
        <div className={hit.kind === "code" ? "row-text mono" : "row-text"}>
          <RowText hit={hit} />
        </div>
        <div className="row-meta">{meta.join(" · ")}</div>
      </div>
      {hit.pinned && <Pin className="row-pin" size={13} strokeWidth={1.75} aria-label="Pinned" />}
      {index < 9 && (
        <kbd className="row-key" aria-hidden="true">
          {keys(`⌘${index + 1}`)}
        </kbd>
      )}
    </div>
  );
});

/** A color swatch, an image thumbnail, or the kind's icon. */
function Leading({ hit, attempt }: { hit: SearchHit; attempt: number }) {
  const color = hit.kind === "color" ? hex(hit.metadata?.rgba) : null;
  if (color) {
    return <span className="leading swatch" style={{ background: color }} aria-hidden="true" />;
  }
  if (hit.kind === "image") {
    return <Thumbnail key={attempt} hash={hit.contentHash} attempt={attempt} />;
  }
  return (
    <span className="leading icon" title={KIND_LABELS[hit.kind]}>
      <KindIcon kind={hit.kind} />
    </span>
  );
}

/** Falls back to the image icon until the thumbnail exists. */
function Thumbnail({ hash, attempt }: { hash: string; attempt: number }) {
  const [failed, setFailed] = useState(false);
  if (failed) {
    return (
      <span className="leading icon" title="Image">
        <KindIcon kind="image" />
      </span>
    );
  }
  return (
    <img
      className="leading thumb"
      src={thumbnailUrl(hash, attempt)}
      alt=""
      draggable={false}
      onError={() => setFailed(true)}
    />
  );
}

/** The snippet, with matches marked, or a stand-in for clips without text. */
function RowText({ hit }: { hit: SearchHit }) {
  if (hit.snippet.length) {
    return hit.snippet.map((span, i) =>
      span.matched ? <mark key={i}>{span.text}</mark> : <span key={i}>{span.text}</span>,
    );
  }
  const m = hit.metadata;
  switch (hit.kind) {
    case "image":
      return <span className="placeholder">Image</span>;
    case "file":
      return <span>{m?.names?.join(", ") || "File"}</span>;
    case "color":
      return <span className="mono">{hex(m?.rgba) ?? "Color"}</span>;
    default:
      return <span className="placeholder">{KIND_LABELS[hit.kind]}</span>;
  }
}

/** What's worth knowing at a glance, by kind. */
function detail(hit: SearchHit): string | null {
  const m = hit.metadata;
  switch (hit.kind) {
    case "link":
      return m?.host ?? null;
    case "image":
      return m?.w && m?.h ? `${m.w} × ${m.h}` : null;
    case "file":
      return m?.count && m.count > 1 ? `${m.count} files` : null;
    case "code":
      return m?.lang ?? "Code";
    default:
      return null;
  }
}
