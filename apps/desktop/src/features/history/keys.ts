// The panel's keyboard shortcuts. The search box keeps focus, so these are read from its
// key events.

export type Action =
  | { type: "move"; by: number }
  | { type: "page"; by: 1 | -1 }
  | { type: "first" }
  | { type: "last" }
  | { type: "paste"; plain: boolean }
  /** ⌘1–9: the result at `index`, from 0. */
  | { type: "quickPaste"; index: number; plain: boolean }
  /** ⌘C: puts the selected clip on the clipboard without pasting it. */
  | { type: "copy"; plain: boolean }
  | { type: "delete" }
  /** ⌘P: pins or unpins the selected clip. */
  | { type: "pin" }
  /** ⌘K: the selected clip's menu. */
  | { type: "menu" }
  /** ⌘[ and ⌘]: the previous or next view, from History through Pinned to each pinboard. */
  | { type: "view"; by: 1 | -1 }
  | { type: "togglePreview" }
  | { type: "hide" };

/** The parts of a KeyboardEvent the shortcuts look at. */
export type Key = Pick<
  KeyboardEvent,
  "key" | "code" | "metaKey" | "ctrlKey" | "altKey" | "shiftKey" | "isComposing" | "keyCode"
>;

/**
 * The action for a key press, or null to let it type. `typing` is whether the search box has
 * text, since Space is then a space.
 */
export function actionFor(e: Key, typing: boolean): Action | null {
  // An input method is composing. The press-and-hold accent picker sets only keyCode 229.
  if (e.isComposing || e.keyCode === 229) return null;
  const { metaKey: cmd, ctrlKey: ctrl, altKey: alt, shiftKey: shift } = e;
  const plain = !cmd && !ctrl && !alt && !shift;

  switch (e.key) {
    case "ArrowDown":
      if (cmd && !ctrl && !alt) return { type: "last" };
      return plain ? { type: "move", by: 1 } : null;
    case "ArrowUp":
      if (cmd && !ctrl && !alt) return { type: "first" };
      return plain ? { type: "move", by: -1 } : null;
    case "PageDown":
      return plain ? { type: "page", by: 1 } : null;
    case "PageUp":
      return plain ? { type: "page", by: -1 } : null;
    case "Enter":
      return !cmd && !ctrl && !shift ? { type: "paste", plain: alt } : null;
    case "Backspace":
      return cmd && !ctrl && !alt && !shift ? { type: "delete" } : null;
    case "Escape":
      return { type: "hide" };
    case " ":
      return plain && !typing ? { type: "togglePreview" } : null;
  }
  // ⌃N and ⌃P move, as in other macOS lists.
  if (ctrl && !cmd && !alt && !shift && (e.key === "n" || e.key === "p")) {
    return { type: "move", by: e.key === "n" ? 1 : -1 };
  }
  if (cmd && !ctrl && !shift) {
    // By physical key, since ⌥ changes what a digit types.
    const digit = /^Digit([1-9])$/.exec(e.code);
    if (digit) return { type: "quickPaste", index: Number(digit[1]) - 1, plain: alt };
    // ⌘Y opens a preview everywhere on macOS, and works while typing.
    if (e.code === "KeyY" && !alt) return { type: "togglePreview" };
    if (e.code === "KeyC") return { type: "copy", plain: alt };
    if (alt) return null;
    if (e.code === "KeyP") return { type: "pin" };
    if (e.code === "KeyK") return { type: "menu" };
    if (e.code === "BracketLeft") return { type: "view", by: -1 };
    if (e.code === "BracketRight") return { type: "view", by: 1 };
  }
  return null;
}
