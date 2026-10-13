import { useEffect, useRef, useState, type KeyboardEvent } from "react";
import { commands, unwrap } from "../../lib/ipc";
import { DEFAULT_SHORTCUT, parts, record } from "./shortcut";

type Props = {
  id?: string;
  /** The shortcut in use, as the plugin writes it. */
  value: string;
  /** Called with the new shortcut once it's registered and saved. */
  onChange: (shortcut: string) => void;
};

/**
 * Shows the shortcut that opens the panel. Clicking it records a new one. While recording,
 * the current shortcut is turned off, so pressing it here doesn't open the panel.
 */
export function ShortcutRecorder({ id, value, onChange }: Props) {
  const ref = useRef<HTMLButtonElement>(null);
  const [recording, setRecording] = useState(false);
  const [held, setHeld] = useState("");
  const [error, setError] = useState<string | null>(null);
  const recordingRef = useRef(false);

  // Turn the shortcut back on if the window goes away mid-recording.
  useEffect(
    () => () => {
      if (recordingRef.current) void commands.suspendShortcut(false);
    },
    [],
  );

  function start() {
    if (recordingRef.current) return;
    recordingRef.current = true;
    setRecording(true);
    setHeld("");
    setError(null);
    void commands.suspendShortcut(true);
  }

  function stop() {
    if (!recordingRef.current) return;
    recordingRef.current = false;
    setRecording(false);
    setHeld("");
    void commands.suspendShortcut(false);
  }

  async function save(shortcut: string) {
    recordingRef.current = false;
    setRecording(false);
    setHeld("");
    try {
      onChange(await unwrap(commands.setShortcut(shortcut)));
      setError(null);
    } catch (e) {
      setError(String(e));
    } finally {
      // Registered again by the change, or the old one put back. This covers anything else.
      void commands.suspendShortcut(false);
    }
  }

  function onKeyDown(e: KeyboardEvent) {
    if (!recording) return;
    e.preventDefault();
    e.stopPropagation();
    const plain = !e.metaKey && !e.ctrlKey && !e.altKey && !e.shiftKey;
    if (plain && (e.key === "Escape" || e.key === "Tab")) {
      stop();
      if (e.key === "Escape") ref.current?.blur();
      return;
    }
    const result = record(e.nativeEvent);
    if (result.type === "partial") {
      setHeld(result.held);
    } else if (result.type === "invalid") {
      setError(result.message);
    } else {
      void save(result.shortcut);
    }
  }

  function onKeyUp(e: KeyboardEvent) {
    if (!recording) return;
    // Shows the modifiers still held.
    const { metaKey, ctrlKey, altKey, shiftKey } = e;
    const result = record({ code: "", metaKey, ctrlKey, altKey, shiftKey });
    if (result.type === "partial") setHeld(result.held);
  }

  const keys = recording ? parts(held) : parts(value);
  return (
    <div className="shortcut">
      <div className="shortcut-controls">
        <button
          id={id}
          ref={ref}
          type="button"
          className={recording ? "shortcut-field recording" : "shortcut-field"}
          aria-label={recording ? "Recording. Type the new shortcut, or Esc to cancel" : "Change shortcut"}
          onClick={start}
          onBlur={stop}
          onKeyDown={onKeyDown}
          onKeyUp={onKeyUp}
        >
          {keys.length ? (
            keys.map((k, i) => <kbd key={i}>{k}</kbd>)
          ) : (
            <span className="muted">Type shortcut…</span>
          )}
        </button>
        {value !== DEFAULT_SHORTCUT && !recording && (
          <button type="button" className="link-button" onClick={() => void save(DEFAULT_SHORTCUT)}>
            Reset
          </button>
        )}
      </div>
      {error && (
        <p className="form-error" role="alert">
          {error}
        </p>
      )}
    </div>
  );
}
