/** The keyboard shortcuts, along the bottom of the panel. */
export function Hints({ typing, previewOpen }: { typing: boolean; previewOpen: boolean }) {
  const hints: [string, string][] = [
    ["↩", "Paste"],
    ["⌥↩", "Plain text"],
    ["⌘1–9", "Quick paste"],
    // Space types a space once there's text, so ⌘Y is shown then.
    [typing ? "⌘Y" : "Space", previewOpen ? "Hide preview" : "Preview"],
    ["⌘⌫", "Delete"],
    ["esc", "Close"],
  ];
  return (
    <footer className="hints" aria-hidden="true">
      {hints.map(([keys, label]) => (
        <span key={label} className="hint">
          <kbd>{keys}</kbd>
          {label}
        </span>
      ))}
    </footer>
  );
}
