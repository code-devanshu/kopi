import { Check } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { FormError } from "../../components/Form";
import { cleanCode, formatCode, timeLeft } from "../../features/sync/sync";
import { commands, events, unwrap, type NearbyComputer, type SyncStatus } from "../../lib/ipc";

/**
 * Pairing with another computer. Both show a code and list each other. On one of them the user
 * picks the other and types the code it shows. Pairing mode lasts while the sheet is open.
 */
export function PairingSheet({ status, onClose }: { status: SyncStatus; onClose: () => void }) {
  const [picked, setPicked] = useState<NearbyComputer | null>(null);
  const [code, setCode] = useState("");
  const [busy, setBusy] = useState(false);
  const [paired, setPaired] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [now, setNow] = useState(() => Date.now());
  const codeRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    unwrap(commands.startPairing()).catch((e) => setError(`Couldn't start pairing. ${e}`));
    const listeners = [
      events.syncPaired.listen(({ payload }) => setPaired(payload.name)),
      events.syncPairingFailed.listen(() =>
        setNotice("A pairing attempt failed, perhaps with a wrong code. This computer shows a new code now."),
      ),
    ];
    const clock = setInterval(() => setNow(Date.now()), 1000);
    return () => {
      clearInterval(clock);
      listeners.forEach((l) => void l.then((stop) => stop()));
      void commands.stopPairing();
    };
  }, []);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  const pairing = status.pairing;
  const nearby = pairing?.nearby ?? [];
  // The computer picked may have closed its sheet.
  const target = picked && nearby.find((n) => n.id === picked.id);

  async function pair() {
    if (!target || code.length !== 6) return;
    setBusy(true);
    setError(null);
    try {
      setPaired(await unwrap(commands.pairWith(target.id, code)));
    } catch (e) {
      setError(String(e));
      setCode("");
      codeRef.current?.focus();
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="sheet-backdrop" onMouseDown={(e) => e.target === e.currentTarget && onClose()}>
      <div className="sheet" role="dialog" aria-modal="true" aria-labelledby="pair-title">
        <h2 id="pair-title" className="sheet-title">
          Add a computer
        </h2>
        {paired ? (
          <>
            <p className="pair-done">
              <Check size={16} strokeWidth={2.25} aria-hidden="true" /> Paired with <strong>{paired}</strong>.
            </p>
            <p className="muted">
              Your clips start syncing now. Any other computers in your group find it the next time they sync
              with this one.
            </p>
            <div className="button-row end">
              <button type="button" className="button primary" autoFocus onClick={onClose}>
                Done
              </button>
            </div>
          </>
        ) : (
          <>
            <p className="muted">
              On your other computer, open Kopi's Settings → Sync, turn sync on and choose Add a computer…. Then,
              on either computer, pick the other one and type the code it shows.
            </p>

            <section className="pair-part">
              <h3 className="pair-heading">This computer's code</h3>
              <p className="pair-code" aria-live="polite">
                {pairing ? formatCode(pairing.code) : "··· ···"}
              </p>
              {pairing && (
                <p className="muted pair-hint">
                  A new code in {timeLeft(pairing.expiresAt, now)}, and after every attempt.
                </p>
              )}
            </section>

            <section className="pair-part">
              <h3 className="pair-heading">Computers nearby</h3>
              {nearby.length === 0 ? (
                <p className="muted pair-hint">Looking for computers with Add a computer… open…</p>
              ) : (
                <div className="nearby" role="radiogroup" aria-label="Computers nearby">
                  {nearby.map((n) => (
                    <button
                      key={n.id}
                      type="button"
                      role="radio"
                      aria-checked={target?.id === n.id}
                      className="nearby-item"
                      onClick={() => {
                        setPicked(n);
                        setError(null);
                        requestAnimationFrame(() => codeRef.current?.focus());
                      }}
                    >
                      {n.name}
                    </button>
                  ))}
                </div>
              )}
              {target && (
                <form
                  className="pair-form"
                  onSubmit={(e) => {
                    e.preventDefault();
                    void pair();
                  }}
                >
                  <label htmlFor="pair-code-input">Code shown on {target.name}</label>
                  <input
                    id="pair-code-input"
                    ref={codeRef}
                    className="text-field mono pair-code-input"
                    inputMode="numeric"
                    autoComplete="off"
                    spellCheck={false}
                    placeholder="000 000"
                    value={formatCode(code)}
                    disabled={busy}
                    onChange={(e) => setCode(cleanCode(e.target.value))}
                  />
                  <button type="submit" className="button primary" disabled={busy || code.length !== 6}>
                    {busy ? "Pairing…" : "Pair"}
                  </button>
                </form>
              )}
            </section>

            {notice && <p className="pair-notice">{notice}</p>}
            <FormError message={error} />
            <div className="button-row end">
              <button type="button" className="button" onClick={onClose}>
                Cancel
              </button>
            </div>
          </>
        )}
      </div>
    </div>
  );
}
