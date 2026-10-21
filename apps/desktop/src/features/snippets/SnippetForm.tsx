import { useRef, useState, type KeyboardEvent } from "react";
import type { Snippet } from "../../lib/ipc";
import { keys } from "../../lib/platform";

type Props = {
  snippet: Snippet;
  /** The `{input}` labels, in order. */
  labels: string[];
  /** Whether it pastes or only copies once filled in. */
  action: "paste" | "copy";
  onSubmit: (inputs: Record<string, string>) => void;
  onCancel: () => void;
};

/** One field per `{input}` label. Return in the last one pastes, Esc goes back. */
export function SnippetForm({ snippet, labels, action, onSubmit, onCancel }: Props) {
  const [values, setValues] = useState<Record<string, string>>({});
  const fields = useRef<(HTMLInputElement | null)[]>([]);

  function onKeyDown(e: KeyboardEvent, index: number) {
    // The panel's shortcuts don't apply while filling in.
    e.stopPropagation();
    if (e.nativeEvent.isComposing) return;
    if (e.key === "Escape") {
      e.preventDefault();
      onCancel();
    } else if (e.key === "Enter") {
      e.preventDefault();
      if (index < labels.length - 1 && !e.metaKey && !e.ctrlKey) fields.current[index + 1]?.focus();
      else onSubmit(values);
    }
  }

  return (
    <form
      className="snippet-form"
      aria-label={`Fill in ${snippet.name}`}
      onSubmit={(e) => {
        e.preventDefault();
        onSubmit(values);
      }}
    >
      <h2 className="editor-title">{snippet.name}</h2>
      {labels.map((label, i) => (
        <label key={label} className="editor-field">
          <span className="editor-label">{label}</span>
          <input
            ref={(el) => {
              fields.current[i] = el;
            }}
            className="editor-input"
            value={values[label] ?? ""}
            autoFocus={i === 0}
            spellCheck={false}
            autoComplete="off"
            onChange={(e) => setValues((v) => ({ ...v, [label]: e.target.value }))}
            onKeyDown={(e) => onKeyDown(e, i)}
          />
        </label>
      ))}
      <div className="button-row">
        <button type="button" className="button" onClick={onCancel}>
          Cancel
        </button>
        <button type="submit" className="button primary">
          {action === "paste" ? "Paste" : "Copy"} <kbd>{keys("↩")}</kbd>
        </button>
      </div>
    </form>
  );
}
