import { History as HistoryIcon, Pin, Plus } from "lucide-react";
import { useEffect, useRef, useState, type PointerEvent, type ReactNode } from "react";
import type { Pinboard } from "../../lib/ipc";
import type { View } from "../history/history";
import { nextColor } from "./boards";
import type { Drag } from "./useDrag";

/** A pinboard name being typed: a new one, or a rename. */
export type Editing =
  | {
      type: "new";
      /** A clip to put on the new pinboard, when it's made from the clip's menu. */
      addClip?: number;
    }
  | { type: "rename"; id: number };

type Props = {
  boards: Pinboard[];
  view: View;
  editing: Editing | null;
  drag: Drag | null;
  /** Whether what's being dragged can be dropped on `target`. */
  canDrop: (target: View) => boolean;
  onView: (view: View) => void;
  onNew: () => void;
  onRename: (id: number) => void;
  onCommitName: (name: string) => void;
  onCancelName: () => void;
  onBoardMenu: (board: Pinboard, x: number, y: number) => void;
  onBoardPointerDown: (e: PointerEvent, board: Pinboard) => void;
};

/** History, Pinned and the pinboards, under the search box. Clips are dropped on the tabs. */
export function Switcher({
  boards,
  view,
  editing,
  drag,
  canDrop,
  onView,
  onNew,
  onRename,
  onCommitName,
  onCancelName,
  onBoardMenu,
  onBoardPointerDown,
}: Props) {
  const ref = useRef<HTMLDivElement>(null);

  // Keep the current tab in view as ⌘[ and ⌘] move through them.
  useEffect(() => {
    ref.current
      ?.querySelector('[aria-selected="true"]')
      ?.scrollIntoView({ block: "nearest", inline: "nearest" });
  }, [view, boards]);

  const tab = (target: View, label: string, content: ReactNode, extra: Partial<TabProps> = {}) => (
    <Tab
      key={String(target)}
      target={target}
      label={label}
      selected={view === target}
      dropping={drag !== null && drag.over === target && canDrop(target)}
      dragged={drag?.item.type === "board" && drag.item.id === target}
      onClick={() => onView(target)}
      {...extra}
    >
      {content}
    </Tab>
  );

  return (
    <div ref={ref} className="switcher" role="tablist" aria-label="Views">
      {tab(
        "history",
        "History",
        <>
          <HistoryIcon size={13} strokeWidth={1.9} aria-hidden="true" />
          History
        </>,
      )}
      {tab(
        "pinned",
        "Pinned",
        <>
          <Pin size={13} strokeWidth={1.9} aria-hidden="true" />
          Pinned
        </>,
      )}
      <span className="switcher-divider" aria-hidden="true" />
      {boards.map((board) =>
        editing?.type === "rename" && editing.id === board.id ? (
          <NameInput key={board.id} initial={board.name} color={board.color} onCommit={onCommitName} onCancel={onCancelName} />
        ) : (
          tab(
            board.id,
            board.name,
            <>
              <span className="dot" style={{ background: board.color ?? undefined }} aria-hidden="true" />
              <span className="tab-name">{board.name}</span>
              {board.clipCount > 0 && <span className="tab-count">{board.clipCount}</span>}
            </>,
            {
              onContextMenu: (x, y) => onBoardMenu(board, x, y),
              onDoubleClick: () => onRename(board.id),
              onPointerDown: (e) => onBoardPointerDown(e, board),
            },
          )
        ),
      )}
      {editing?.type === "new" ? (
        <NameInput initial="" color={nextColor(boards)} onCommit={onCommitName} onCancel={onCancelName} />
      ) : (
        <button
          type="button"
          className="tab tab-add"
          title="New pinboard"
          aria-label="New pinboard"
          tabIndex={-1}
          onMouseDown={(e) => e.preventDefault()}
          onClick={onNew}
        >
          <Plus size={14} strokeWidth={2} aria-hidden="true" />
          {boards.length === 0 && <span>New pinboard</span>}
        </button>
      )}
    </div>
  );
}

type TabProps = {
  target: View;
  label: string;
  selected: boolean;
  dropping: boolean;
  dragged: boolean;
  children: ReactNode;
  onClick: () => void;
  onContextMenu?: (x: number, y: number) => void;
  onDoubleClick?: () => void;
  onPointerDown?: (e: PointerEvent) => void;
};

function Tab({ target, label, selected, dropping, dragged, children, onClick, onContextMenu, onDoubleClick, onPointerDown }: TabProps) {
  const classes = ["tab"];
  if (dropping) classes.push("dropping");
  if (dragged) classes.push("dragged");
  return (
    <button
      type="button"
      role="tab"
      className={classes.join(" ")}
      aria-selected={selected}
      title={label}
      tabIndex={-1}
      data-drop={String(target)}
      // The search box keeps focus.
      onMouseDown={(e) => e.preventDefault()}
      onClick={onClick}
      onDoubleClick={onDoubleClick}
      onPointerDown={onPointerDown}
      onContextMenu={(e) => {
        e.preventDefault();
        onContextMenu?.(e.clientX, e.clientY);
      }}
    >
      {children}
    </button>
  );
}

/** Typing a pinboard's name. Return or clicking away saves it, Esc cancels. */
function NameInput({
  initial,
  color,
  onCommit,
  onCancel,
}: {
  initial: string;
  color: string | null;
  onCommit: (name: string) => void;
  onCancel: () => void;
}) {
  const [value, setValue] = useState(initial);
  // Return blurs the input as it goes, which mustn't save a second time.
  const done = useRef(false);
  const finish = (save: boolean) => {
    if (done.current) return;
    done.current = true;
    if (save && value.trim()) onCommit(value);
    else onCancel();
  };
  return (
    <span className="tab editing">
      <span className="dot" style={{ background: color ?? undefined }} aria-hidden="true" />
      <input
        className="tab-input"
        aria-label="Pinboard name"
        placeholder="Pinboard name"
        value={value}
        maxLength={64}
        size={Math.max(10, value.length + 1)}
        autoFocus
        spellCheck={false}
        autoComplete="off"
        onFocus={(e) => e.currentTarget.select()}
        onChange={(e) => setValue(e.target.value)}
        onBlur={() => finish(true)}
        onKeyDown={(e) => {
          // The panel's shortcuts don't apply while naming.
          e.stopPropagation();
          if (e.key === "Enter" && !e.nativeEvent.isComposing) {
            e.preventDefault();
            finish(true);
          } else if (e.key === "Escape") {
            e.preventDefault();
            finish(false);
          }
        }}
      />
    </span>
  );
}
