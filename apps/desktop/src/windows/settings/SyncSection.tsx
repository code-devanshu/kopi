import { Laptop, Monitor, TextQuote } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { FormError, Group, Row, Switch } from "../../components/Form";
import { KindIcon } from "../../components/KindIcon";
import { deviceStatus, SYNC_KINDS } from "../../features/sync/sync";
import { KIND_LABELS } from "../../lib/format";
import { commands, events, unwrap, type ClipKind, type SyncDevice, type SyncStatus } from "../../lib/ipc";
import { PairingSheet } from "./PairingSheet";

/** How often "Last synced 5 minutes ago" is brought up to date. */
const CLOCK_MS = 30_000;

export function SyncSection() {
  const [status, setStatus] = useState<SyncStatus | null>(null);
  /** The name being typed, until it's saved. */
  const [draftName, setDraftName] = useState<string | null>(null);
  const [switching, setSwitching] = useState(false);
  const [pairing, setPairing] = useState(false);
  const [removing, setRemoving] = useState<SyncDevice | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [now, setNow] = useState(() => Date.now());

  const load = useCallback(
    () =>
      unwrap(commands.syncStatus()).then(
        (next) => {
          setStatus(next);
          setNow(Date.now());
        },
        (e) => setError(`Couldn't load the sync settings. ${e}`),
      ),
    [],
  );

  useEffect(() => {
    void load();
    const listener = events.syncChanged.listen(() => void load());
    const clock = setInterval(() => setNow(Date.now()), CLOCK_MS);
    return () => {
      clearInterval(clock);
      void listener.then((stop) => stop());
    };
  }, [load]);

  async function toggle(on: boolean) {
    setSwitching(true);
    setError(null);
    try {
      await unwrap(commands.setSyncEnabled(on));
    } catch (e) {
      setError(on ? `Couldn't turn sync on. ${e}` : `Couldn't turn sync off. ${e}`);
    } finally {
      setSwitching(false);
      void load();
    }
  }

  function saveName() {
    const name = draftName;
    setDraftName(null);
    if (!status || name === null || name.trim() === status.deviceName) return;
    setStatus({ ...status, deviceName: name.trim() });
    unwrap(commands.setSyncDeviceName(name)).then(
      () => load(),
      (e) => {
        setError(`Couldn't rename this computer. ${e}`);
        void load();
      },
    );
  }

  function toggleKind(kind: ClipKind, on: boolean) {
    if (!status) return;
    const was = status;
    const kinds = SYNC_KINDS.filter((k) => (k === kind ? on : status.kinds.includes(k)));
    setStatus({ ...status, kinds });
    unwrap(commands.setSyncKinds(kinds)).catch((e) => {
      setStatus(was);
      setError(`Couldn't change what syncs. ${e}`);
    });
  }

  function toggleSnippets(on: boolean) {
    if (!status) return;
    const was = status;
    setStatus({ ...status, snippets: on });
    unwrap(commands.setSyncSnippets(on)).catch((e) => {
      setStatus(was);
      setError(`Couldn't change what syncs. ${e}`);
    });
  }

  function remove(device: SyncDevice) {
    setRemoving(null);
    unwrap(commands.removeSyncDevice(device.id)).then(
      () => load(),
      (e) => setError(`Couldn't remove ${device.name}. ${e}`),
    );
  }

  const on = status?.enabled ?? false;

  return (
    <>
      <FormError message={error} />
      <Group footer="Over your local network only. Clips go straight between your computers, encrypted, and never through a server.">
        <Row
          label="Sync with your other computers"
          description="History, pins, pinboards and snippets, on every Mac and Windows PC you pair."
          htmlFor="sync-on"
        >
          <Switch id="sync-on" checked={on} disabled={status === null || switching} onChange={(next) => void toggle(next)} />
        </Row>
        {on && (
          <Row label="This computer's name" description="What your other computers call this one." htmlFor="sync-name">
            <input
              id="sync-name"
              className="text-field"
              value={draftName ?? status?.deviceName ?? ""}
              maxLength={64}
              spellCheck={false}
              onChange={(e) => setDraftName(e.target.value)}
              onBlur={saveName}
              onKeyDown={(e) => {
                if (e.key === "Enter") e.currentTarget.blur();
                if (e.key === "Escape") setDraftName(null);
              }}
            />
          </Row>
        )}
      </Group>

      {on && status?.problem === "localNetworkDenied" && (
        <div className="callout" role="status">
          <p>
            <strong>Kopi can't reach your local network.</strong> macOS asks once whether Kopi may find devices
            on your network. Turn Kopi on in Privacy &amp; Security → Local Network, then turn sync off and on
            again here.
          </p>
          <button type="button" className="button" onClick={() => void commands.openLocalNetworkSettings()}>
            Open Local Network Settings
          </button>
        </div>
      )}
      {on && status?.problem === "discovery" && (
        <div className="callout" role="status">
          <p>
            <strong>Kopi can't look for your other computers.</strong> Something on this computer is
            blocking it. Try turning sync off and on again.
          </p>
        </div>
      )}

      {on && status && (
        <>
          <Group
            title="Computers"
            footer="Removing a computer stops every computer in the group syncing with it. It keeps the clips it already has."
          >
            {status.devices.length === 0 && (
              <p className="group-empty muted">No other computers yet. Add one to start syncing.</p>
            )}
            {status.devices.map((device) => {
              const Icon = device.os === "windows" ? Monitor : Laptop;
              return (
                <div key={device.id}>
                  <div className="app-row">
                    <Icon className="device-icon" size={16} strokeWidth={1.75} aria-hidden="true" />
                    <div className="app-row-text">
                      <span className="app-row-name">
                        {device.name}
                        <span className="tag">{device.os === "windows" ? "Windows" : "Mac"}</span>
                      </span>
                      <span className={device.connected ? "device-status connected" : "device-status muted"}>
                        {deviceStatus(device, on, now)}
                      </span>
                    </div>
                    <button
                      type="button"
                      className="button"
                      disabled={removing !== null}
                      onClick={() => setRemoving(device)}
                    >
                      Remove…
                    </button>
                  </div>
                  {removing?.id === device.id && (
                    <div className="confirm" role="alertdialog" aria-labelledby="remove-text">
                      <p id="remove-text">
                        Stop syncing with {device.name}? It keeps the clips it already has. To sync with it
                        again, pair it again.
                      </p>
                      <div className="button-row">
                        <button type="button" className="button" autoFocus onClick={() => setRemoving(null)}>
                          Cancel
                        </button>
                        <button type="button" className="button danger solid" onClick={() => remove(device)}>
                          Remove
                        </button>
                      </div>
                    </div>
                  )}
                </div>
              );
            })}
            <div className="app-row add">
              <button type="button" className="button" onClick={() => setPairing(true)}>
                Add a computer…
              </button>
            </div>
          </Group>

          <Group
            title="What to sync"
            footer="File clips stay on the computer they were copied on: a file's path means nothing on another computer. Ticking Snippets again brings the ones made elsewhere meanwhile."
          >
            <div className="kind-grid">
              {SYNC_KINDS.map((kind) => (
                <label key={kind} className="kind-option">
                  <input
                    type="checkbox"
                    checked={status.kinds.includes(kind)}
                    onChange={(e) => toggleKind(kind, e.target.checked)}
                  />
                  <KindIcon kind={kind} />
                  {KIND_LABELS[kind]}
                </label>
              ))}
              <label className="kind-option">
                <input
                  type="checkbox"
                  checked={status.snippets}
                  onChange={(e) => toggleSnippets(e.target.checked)}
                />
                <TextQuote size={15} strokeWidth={1.75} aria-hidden="true" />
                Snippets
              </label>
              <label className="kind-option disabled">
                <input type="checkbox" checked={false} disabled />
                <KindIcon kind="file" />
                {KIND_LABELS.file}
              </label>
            </div>
          </Group>

          <p className="settings-note muted">
            Your computers sync while they're on the same network with Kopi running. One that was away catches
            up when it's back. Guest Wi-Fi and some office networks keep computers apart, so they can't find
            each other. On Windows, keep the network set to Private, and allow Kopi when Windows Defender
            Firewall asks.
          </p>
        </>
      )}

      {pairing && status && <PairingSheet status={status} onClose={() => setPairing(false)} />}
    </>
  );
}
