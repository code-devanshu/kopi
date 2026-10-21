import { Check } from "lucide-react";
import { useLayoutEffect, useRef, useState, type KeyboardEvent, type ReactNode } from "react";
import { keys } from "../lib/platform";

export type MenuEntry =
  | {
      type: "item";
      label: string;
      /** A color dot before the label. */
      color?: string | null;
      shortcut?: string;
      /** A check mark column is shown when any item sets this. */
      checked?: boolean;
      danger?: boolean;
      disabled?: boolean;
      /** Asks for a second click, showing this label, before running. */
      confirm?: string;
      run: () => void;
    }
  | { type: "separator" }
  | { type: "heading"; label: string }
  /** Something of its own, such as a row of color swatches. It runs its own clicks. */
  | { type: "custom"; key: string; node: ReactNode };

type Props = {
  label: string;
  /** Where it opens, in window coordinates. It moves in to stay inside the window. */
  x: number;
  y: number;
  entries: MenuEntry[];
  /** The entry that's active when it opens, so Return picks it. None by default. */
  initialActive?: number;
  onClose: () => void;
};

/** Window edges the menu keeps clear of. */
const MARGIN = 6;

/**
 * A menu that takes focus while open, so arrow keys, Return and Esc work in it. Clicking
 * outside closes it. The caller puts focus back where it was in `onClose`.
 */
export function Menu({ label, x, y, entries, initialActive = -1, onClose }: Props) {
  const ref = useRef<HTMLDivElement>(null);
  const [pos, setPos] = useState({ left: x, top: y });
  const [active, setActive] = useState(initialActive);
  const [confirming, setConfirming] = useState(-1);
  const items = entries.flatMap((e, i) => (e.type === "item" && !e.disabled ? [i] : []));
  const checks = entries.some((e) => e.type === "item" && e.checked !== undefined);

  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    const { width, height } = el.getBoundingClientRect();
    setPos({
      left: Math.max(MARGIN, Math.min(x, window.innerWidth - width - MARGIN)),
      top: Math.max(MARGIN, Math.min(y, window.innerHeight - height - MARGIN)),
    });
    el.focus();
  }, [x, y]);

  function run(index: number) {
    const entry = entries[index];
    if (entry?.type !== "item" || entry.disabled) return;
    if (entry.confirm && confirming !== index) {
      setConfirming(index);
      setActive(index);
      return;
    }
    onClose();
    entry.run();
  }

  function step(by: number) {
    if (!items.length) return;
    const at = items.indexOf(active);
    const next = at < 0 ? (by > 0 ? 0 : items.length - 1) : (at + by + items.length) % items.length;
    setActive(items[next]);
  }

  function onKeyDown(e: KeyboardEvent) {
    // The panel's shortcuts wait until the menu is closed.
    e.stopPropagation();
    switch (e.key) {
      case "ArrowDown":
        e.preventDefault();
        return step(1);
      case "ArrowUp":
        e.preventDefault();
        return step(-1);
      case "Home":
        e.preventDefault();
        return setActive(items[0] ?? -1);
      case "End":
        e.preventDefault();
        return setActive(items.at(-1) ?? -1);
      case "Enter":
      case " ":
        e.preventDefault();
        return run(active);
      case "Escape":
      case "Tab":
        e.preventDefault();
        return onClose();
    }
  }

  return (
    <>
      <div
        className="menu-backdrop"
        onMouseDown={(e) => {
          e.preventDefault();
          onClose();
        }}
        onContextMenu={(e) => {
          e.preventDefault();
          onClose();
        }}
      />
      <div
        ref={ref}
        className="menu"
        role="menu"
        aria-label={label}
        tabIndex={-1}
        style={pos}
        aria-activedescendant={active >= 0 ? `menu-${active}` : undefined}
        onKeyDown={onKeyDown}
        onContextMenu={(e) => e.preventDefault()}
      >
        {entries.map((entry, i) => {
          switch (entry.type) {
            case "separator":
              return <div key={i} className="menu-separator" role="separator" />;
            case "heading":
              return (
                <div key={i} className="menu-heading" role="presentation">
                  {entry.label}
                </div>
              );
            case "custom":
              return <div key={entry.key}>{entry.node}</div>;
          }
          const classes = ["menu-item"];
          if (i === active) classes.push("active");
          if (entry.danger) classes.push("danger");
          return (
            <div
              key={i}
              id={`menu-${i}`}
              className={classes.join(" ")}
              role={entry.checked === undefined ? "menuitem" : "menuitemcheckbox"}
              aria-checked={entry.checked}
              aria-disabled={entry.disabled}
              onMouseEnter={() => !entry.disabled && setActive(i)}
              onMouseDown={(e) => e.preventDefault()}
              onClick={() => run(i)}
            >
              {checks && (
                <span className="menu-check" aria-hidden="true">
                  {entry.checked && <Check size={13} strokeWidth={2.25} />}
                </span>
              )}
              {entry.color !== undefined && (
                <span className="dot" style={{ background: entry.color ?? undefined }} aria-hidden="true" />
              )}
              <span className="menu-label">{confirming === i && entry.confirm ? entry.confirm : entry.label}</span>
              {entry.shortcut && <kbd className="menu-shortcut">{keys(entry.shortcut)}</kbd>}
            </div>
          );
        })}
      </div>
    </>
  );
}
