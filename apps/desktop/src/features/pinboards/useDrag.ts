// Dragging clips onto pinboards, pinboards along the switcher, and snippets up and down their
// list. It follows the pointer instead of using HTML drag and drop, which the Tauri webview
// takes over for file drops.
import { useCallback, useEffect, useRef, useState, type PointerEvent as ReactPointerEvent } from "react";
import type { View } from "../history/history";

/** What's being dragged. `label` follows the pointer. */
export type DragItem = { type: "clip" | "board" | "snippet"; id: number; label: string };

/** Where something can be dropped: a tab, or a snippet's row. */
export type DropTarget = View | { snippet: number };

export type Drag = {
  item: DragItem;
  x: number;
  y: number;
  /** What's under the pointer, from the nearest `data-drop` element. */
  over: DropTarget | null;
};

/** How far the pointer moves before a press becomes a drag, so clicks still work. */
const THRESHOLD = 5;

/** The drag in progress, and `start` for a source's onPointerDown. */
export function useDrag(onDrop: (item: DragItem, target: DropTarget) => void) {
  const [drag, setDrag] = useState<Drag | null>(null);
  const onDropRef = useRef(onDrop);
  const stopRef = useRef<() => void>(() => {});

  useEffect(() => {
    onDropRef.current = onDrop;
  }, [onDrop]);

  useEffect(() => () => stopRef.current(), []);

  const start = useCallback((e: ReactPointerEvent, item: DragItem) => {
    if (e.button !== 0 || e.metaKey || e.ctrlKey) return;
    const x0 = e.clientX;
    const y0 = e.clientY;
    let active = false;

    const move = (ev: PointerEvent) => {
      if (!active && Math.hypot(ev.clientX - x0, ev.clientY - y0) < THRESHOLD) return;
      if (!active) document.documentElement.classList.add("dragging");
      active = true;
      setDrag({ item, x: ev.clientX, y: ev.clientY, over: dropAt(ev.clientX, ev.clientY) });
    };
    const up = (ev: PointerEvent) => {
      const wasActive = active;
      stop();
      if (!wasActive) return;
      // The release would click whatever is under it.
      swallowNextClick();
      const target = dropAt(ev.clientX, ev.clientY);
      if (target !== null) onDropRef.current(item, target);
    };
    const stop = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
      window.removeEventListener("pointercancel", stop);
      window.removeEventListener("blur", stop);
      document.documentElement.classList.remove("dragging");
      active = false;
      setDrag(null);
      stopRef.current = () => {};
    };
    stopRef.current();
    stopRef.current = stop;
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
    window.addEventListener("pointercancel", stop);
    window.addEventListener("blur", stop);
  }, []);

  const cancel = useCallback(() => stopRef.current(), []);

  return { drag, start, cancel };
}

/** What the `data-drop` element under the point stands for. */
function dropAt(x: number, y: number): DropTarget | null {
  const el = document.elementFromPoint(x, y)?.closest<HTMLElement>("[data-drop]");
  return parseDrop(el?.dataset.drop);
}

/** A `data-drop` value: a view, or `snippet:<id>`. */
export function parseDrop(value: string | undefined): DropTarget | null {
  if (!value) return null;
  if (value === "history" || value === "pinned" || value === "snippets") return value;
  const snippet = /^snippet:(\d+)$/.exec(value);
  if (snippet) return { snippet: Number(snippet[1]) };
  const id = Number(value);
  return Number.isInteger(id) ? id : null;
}

function swallowNextClick() {
  const swallow = (e: MouseEvent) => {
    e.stopPropagation();
    e.preventDefault();
  };
  window.addEventListener("click", swallow, { capture: true, once: true });
  // A release with no click after it mustn't eat the next real one.
  setTimeout(() => window.removeEventListener("click", swallow, { capture: true }), 0);
}
