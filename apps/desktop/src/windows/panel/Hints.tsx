type Props = {
  typing: boolean;
  previewOpen: boolean;
  /** In a pinboard, ⌘⌫ takes the clip off it instead of deleting it. */
  inBoard: boolean;
};

/** The keyboard shortcuts, along the bottom of the panel. */
export function Hints({ typing, previewOpen, inBoard }: Props) {
  const hints: [string, string][] = [
    ["↩", "Paste"],
    ["⌥↩", "Plain text"],
    ["⌘1–9", "Quick paste"],
    // Space types a space once there's text, so ⌘Y is shown then.
    [typing ? "⌘Y" : "Space", previewOpen ? "Hide preview" : "Preview"],
    ["⌘P", "Pin"],
    ["⌘K", "Actions"],
    ["⌘⌫", inBoard ? "Remove" : "Delete"],
    ["esc", "Close"],
  ];
  return (
    <footer className="hints" aria-hidden="true">
      {hints.map(([keys, label]) => (
        <span key={keys} className="hint">
          <kbd>{keys}</kbd>
          {label}
        </span>
      ))}
    </footer>
  );
}
