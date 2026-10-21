import { keys } from "../../lib/platform";

type Props = {
  typing: boolean;
  previewOpen: boolean;
  /** In a pinboard, ⌘⌫ takes the clip off it instead of deleting it. */
  inBoard: boolean;
  /** The Snippets tab has its own keys. */
  inSnippets: boolean;
};

/** The keyboard shortcuts, along the bottom of the panel. */
export function Hints({ typing, previewOpen, inBoard, inSnippets }: Props) {
  // Space types a space once there's text, so ⌘Y is shown then.
  const preview: [string, string] = [typing ? "⌘Y" : "Space", previewOpen ? "Hide preview" : "Preview"];
  const hints: [string, string][] = inSnippets
    ? [
        ["↩", "Paste"],
        ["⌘1–9", "Quick paste"],
        ["⌘N", "New"],
        ["⌘E", "Edit"],
        preview,
        ["⌘K", "Actions"],
        ["⌘⌫", "Delete"],
        ["esc", "Close"],
      ]
    : [
        ["↩", "Paste"],
        ["⌥↩", "Plain text"],
        ["⌘1–9", "Quick paste"],
        preview,
        ["⌘P", "Pin"],
        ["⌘K", "Actions"],
        ["⌘⌫", inBoard ? "Remove" : "Delete"],
        ["esc", "Close"],
      ];
  return (
    <footer className="hints" aria-hidden="true">
      {hints.map(([shortcut, label]) => (
        <span key={shortcut} className="hint">
          <kbd>{keys(shortcut)}</kbd>
          {label}
        </span>
      ))}
    </footer>
  );
}
