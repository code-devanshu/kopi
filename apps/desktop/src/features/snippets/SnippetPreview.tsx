import { useEffect, useMemo, useState, type ReactNode } from "react";
import { dateTime } from "../../lib/format";
import { commands, unwrap, type Snippet } from "../../lib/ipc";
import { parse, placeholderLabel, usesClipboard, valueOf, type Placeholder, type Token } from "./template";

/** The selected snippet: its text with the placeholders marked, and what it would paste now. */
export function SnippetPreview({ snippet, now, version }: { snippet: Snippet; now: number; version: number }) {
  const tokens = useMemo(() => parse(snippet.body), [snippet.body]);
  const clipboard = useClipboard(usesClipboard(tokens), version);
  const hasPlaceholders = tokens.some((t) => t.type !== "text");
  const ctx = { now: new Date(now), clipboard, inputs: {} };

  return (
    <aside className="preview" aria-label="Preview">
      <div className="preview-body" key={snippet.id}>
        {snippet.body ? (
          <pre className="preview-text">
            {tokens.map((t, i) => (t.type === "text" ? t.text : <Chip key={i} token={t} />))}
          </pre>
        ) : (
          <p className="muted">No text</p>
        )}
        {hasPlaceholders && (
          <>
            <h3 className="preview-heading">Pastes</h3>
            <pre className="preview-text">
              {tokens.map((t, i) => (
                <Filled key={i} token={t} value={valueOf(t, ctx)} />
              ))}
            </pre>
          </>
        )}
      </div>
      <dl className="preview-meta">
        <dt>Name</dt>
        <dd>{snippet.name}</dd>
        {snippet.keyword && (
          <>
            <dt>Keyword</dt>
            <dd className="mono">{snippet.keyword}</dd>
          </>
        )}
        {snippet.useCount > 0 && (
          <>
            <dt>Used</dt>
            <dd>{snippet.useCount === 1 ? "Once" : `${snippet.useCount} times`}</dd>
          </>
        )}
        {snippet.lastUsedAt !== null && (
          <>
            <dt>Last used</dt>
            <dd>{dateTime(snippet.lastUsedAt)}</dd>
          </>
        )}
        <dt>Edited</dt>
        <dd>{dateTime(snippet.updatedAt)}</dd>
      </dl>
    </aside>
  );
}

/** The newest text in the history, loaded only when the snippet uses `{clipboard}`. */
function useClipboard(needed: boolean, version: number): string | null {
  const [text, setText] = useState<string | null>(null);
  useEffect(() => {
    if (!needed) return;
    let live = true;
    unwrap(commands.clipboardText()).then(
      (t) => live && setText(t),
      () => live && setText(null),
    );
    return () => {
      live = false;
    };
  }, [needed, version]);
  return needed ? text : null;
}

function Chip({ token }: { token: Placeholder }) {
  return (
    <span className={`chip chip-${token.type}`} title={token.raw}>
      {placeholderLabel(token)}
    </span>
  );
}

/** A token as it would paste: text as it is, a blank to fill in as its chip. */
function Filled({ token, value }: { token: Token; value: string }): ReactNode {
  if (token.type === "text") return token.text;
  if (token.type === "input") return <Chip token={token} />;
  if (token.type === "cursor") return <span className="caret" aria-label="Cursor" />;
  if (!value) return <span className="muted">(nothing)</span>;
  return <span className="filled">{value}</span>;
}
