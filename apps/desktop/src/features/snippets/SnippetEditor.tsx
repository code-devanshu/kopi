import { ChevronDown, ChevronLeft } from "lucide-react";
import { useEffect, useImperativeHandle, useMemo, useRef, useState, type KeyboardEvent, type Ref } from "react";
import { Menu } from "../../components/Menu";
import { commands, unwrap, type Snippet, type SnippetFields } from "../../lib/ipc";
import { keys, primary } from "../../lib/platform";
import { INSERTABLE, nameFrom, parse } from "./template";

/** How long after the last change the editor saves. */
export const SAVE_DELAY_MS = 400;

export type EditorHandle = {
  /** Saves what's typed now. Resolves to the snippet's id, or null if there's nothing to keep. */
  flush: () => Promise<number | null>;
};

type Props = {
  ref?: Ref<EditorHandle>;
  /** The snippet being edited, or null for a new one. */
  snippet: Snippet | null;
  /** What the fields start with: the snippet's, or a clip's text for Save as snippet. */
  initial: SnippetFields;
  /** Every snippet, to say when a keyword is already used. */
  others: Snippet[];
  /** Esc, ⌘↩ or Done. */
  onDone: () => void;
};

/**
 * Making or editing a snippet, in place of the list. Changes save as they're made, as in
 * Notes. A new snippet is saved once it has a name, and one that's still empty when the editor
 * closes is dropped.
 */
export function SnippetEditor({ ref, snippet, initial, others, onDone }: Props) {
  const [fields, setFields] = useState(initial);
  const [saved, setSaved] = useState<SnippetFields | null>(snippet ? initial : null);
  const [error, setError] = useState<string | null>(null);
  // The saved snippet's id, once there is one. idRef has it for saving.
  const [id, setId] = useState(snippet?.id ?? null);
  const [insertAt, setInsertAt] = useState<{ x: number; y: number } | null>(null);
  const fieldsRef = useRef(fields);
  const savedRef = useRef(saved);
  const idRef = useRef(snippet?.id ?? null);
  const chain = useRef<Promise<void>>(Promise.resolve());
  const timer = useRef<ReturnType<typeof setTimeout>>(undefined);
  const textRef = useRef<HTMLTextAreaElement>(null);
  const markRef = useRef<HTMLDivElement>(null);
  const nameRef = useRef<HTMLInputElement>(null);

  /** Saves the fields, after any save already on its way. `final` names an unnamed snippet
   * after its text, or keeps the name it had, instead of waiting for one. */
  function save(final: boolean): Promise<void> {
    clearTimeout(timer.current);
    chain.current = chain.current.then(async () => {
      const typed = fieldsRef.current;
      let name = typed.name.trim();
      if (!name) {
        if (!final) return;
        name = savedRef.current?.name ?? nameFrom(typed.body);
        // Nothing typed: nothing to keep.
        if (!name) return;
      }
      const next: SnippetFields = { name, keyword: typed.keyword?.trim() || null, body: typed.body };
      if (same(next, savedRef.current)) return;
      try {
        if (idRef.current === null) {
          idRef.current = (await unwrap(commands.snippetCreate(next))).id;
          setId(idRef.current);
        } else {
          await unwrap(commands.snippetUpdate(idRef.current, next));
        }
        savedRef.current = next;
        setSaved(next);
        setError(null);
      } catch (e) {
        setError(String(e));
      }
    });
    return chain.current;
  }

  useImperativeHandle(ref, () => ({
    flush: async () => {
      await save(true);
      return idRef.current;
    },
  }));

  useEffect(() => {
    (snippet || initial.body ? textRef : nameRef).current?.focus();
    // Made from a clip, it's kept straight away, as Save as snippet says.
    if (!snippet && initial.name.trim()) void save(false);
    // Closing some other way, such as another tab, still saves.
    return () => void save(true);
    // Only when it opens and closes.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  function change(patch: Partial<SnippetFields>) {
    const next = { ...fieldsRef.current, ...patch };
    fieldsRef.current = next;
    setFields(next);
    clearTimeout(timer.current);
    timer.current = setTimeout(() => void save(false), SAVE_DELAY_MS);
  }

  function insert(text: string) {
    const area = textRef.current;
    const body = fieldsRef.current.body;
    const from = area?.selectionStart ?? body.length;
    const to = area?.selectionEnd ?? body.length;
    change({ body: body.slice(0, from) + text + body.slice(to) });
    requestAnimationFrame(() => {
      if (!area) return;
      area.focus();
      // A new {input:Name} has its label selected, ready to type over.
      const label = /^\{input:(.+)\}$/.exec(text);
      if (label) area.setSelectionRange(from + 7, from + 7 + label[1].length);
      else area.setSelectionRange(from + text.length, from + text.length);
    });
  }

  function onKeyDown(e: KeyboardEvent) {
    // The panel's shortcuts don't apply while editing.
    e.stopPropagation();
    if (e.nativeEvent.isComposing) return;
    if (e.key === "Escape" || (e.key === "Enter" && primary(e.nativeEvent))) {
      e.preventDefault();
      onDone();
    }
  }

  const keyword = fields.keyword ?? "";
  const clash = keyword.trim()
    ? others.find((o) => o.id !== id && o.keyword?.toLowerCase() === keyword.trim().toLowerCase())
    : undefined;
  const tokens = useMemo(() => parse(fields.body), [fields.body]);
  let status: string | null = null;
  if (error) status = null;
  else if (!fields.name.trim()) {
    if (saved) status = `Keeps the name ${saved.name} until you type one`;
    else if (fields.body) status = "Give it a name to save it";
  } else status = same({ ...fields, name: fields.name.trim(), keyword: keyword.trim() || null }, saved) ? "Saved" : "Saving…";

  return (
    <div className="snippet-editor" onKeyDown={onKeyDown}>
      <div className="editor-head">
        <button type="button" className="link-button editor-back" onClick={onDone}>
          <ChevronLeft size={14} strokeWidth={2} aria-hidden="true" />
          Snippets
        </button>
        <h2 className="editor-title">{snippet ? "Edit snippet" : "New snippet"}</h2>
        <span className="editor-status" role="status">
          {status}
        </span>
        <button type="button" className="button" onClick={onDone} title={keys("⌘↩")}>
          Done
        </button>
      </div>
      <div className="editor-fields">
        <label className="editor-field">
          <span className="editor-label">Name</span>
          <input
            ref={nameRef}
            className="editor-input"
            value={fields.name}
            maxLength={64}
            placeholder="Signature"
            spellCheck={false}
            autoComplete="off"
            onChange={(e) => change({ name: e.target.value })}
          />
        </label>
        <label className="editor-field keyword">
          <span className="editor-label">Keyword</span>
          <input
            className="editor-input mono"
            value={keyword}
            maxLength={32}
            placeholder="sig"
            spellCheck={false}
            autoComplete="off"
            autoCapitalize="off"
            aria-describedby="keyword-hint"
            // A keyword has no spaces.
            onChange={(e) => change({ keyword: e.target.value.replace(/\s/g, "") })}
          />
        </label>
      </div>
      <p id="keyword-hint" className={clash ? "editor-hint warn" : "editor-hint"}>
        {clash ? `Also used by ${clash.name}.` : "Type the keyword in the search box to find it first."}
      </p>
      <div className="editor-text-head">
        <span className="editor-label">Text</span>
        <button
          type="button"
          className="link-button"
          aria-haspopup="menu"
          onMouseDown={(e) => e.preventDefault()}
          onClick={(e) => {
            const r = e.currentTarget.getBoundingClientRect();
            setInsertAt({ x: r.left, y: r.bottom + 4 });
          }}
        >
          Insert
          <ChevronDown size={12} strokeWidth={2} aria-hidden="true" />
        </button>
      </div>
      <div className="editor-text">
        <div ref={markRef} className="editor-marks" aria-hidden="true">
          {tokens.map((t, i) => (t.type === "text" ? t.text : <mark key={i}>{t.raw}</mark>))}
          {/* A last empty line still takes its height. */}
          {"\n "}
        </div>
        <textarea
          ref={textRef}
          className="editor-textarea"
          aria-label="Text"
          value={fields.body}
          placeholder={"Hi {input:Name},\n\nThanks for your order on {date}."}
          spellCheck={false}
          onChange={(e) => change({ body: e.target.value })}
          onScroll={(e) => {
            if (markRef.current) markRef.current.scrollTop = e.currentTarget.scrollTop;
          }}
        />
      </div>
      {error ? (
        <p className="form-error" role="alert">
          Couldn’t save: {error}
        </p>
      ) : (
        <p className="editor-hint">
          {"{date}"}, {"{time}"} and {"{clipboard}"} fill in when it’s pasted. {"{input:Name}"} asks for a Name first.{" "}
          {"{cursor}"} is where the cursor ends up; it works best near the end, since some apps change pasted text.
        </p>
      )}
      {insertAt && (
        <Menu
          label="Insert a placeholder"
          x={insertAt.x}
          y={insertAt.y}
          entries={INSERTABLE.map((p) => ({
            type: "item",
            label: p.label,
            shortcut: p.text,
            run: () => insert(p.text),
          }))}
          onClose={() => {
            setInsertAt(null);
            textRef.current?.focus();
          }}
        />
      )}
    </div>
  );
}

function same(a: SnippetFields, b: SnippetFields | null): boolean {
  return b !== null && a.name === b.name && (a.keyword || null) === (b.keyword || null) && a.body === b.body;
}
