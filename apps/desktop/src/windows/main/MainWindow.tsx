import { useEffect, useState } from "react";
import { commands, unwrap, type AppInfo } from "../../lib/ipc";

// A placeholder until the panel UI lands (#13). It shows that the typed IPC works.
export function MainWindow() {
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [launchAtLogin, setLaunchAtLogin] = useState<boolean | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    commands.appInfo().then(setInfo);
    unwrap(commands.launchAtLogin()).then(setLaunchAtLogin, (e) => setError(String(e)));
  }, []);

  async function toggleLaunchAtLogin(enabled: boolean) {
    setError(null);
    try {
      await unwrap(commands.setLaunchAtLogin(enabled));
      setLaunchAtLogin(enabled);
    } catch (e) {
      setError(String(e));
    }
  }

  return (
    <main className="main">
      <h1>Kopi</h1>
      <p className="muted">{info ? `Version ${info.version} on ${info.os}` : "Loading…"}</p>
      <label className="row">
        <input
          type="checkbox"
          checked={launchAtLogin ?? false}
          disabled={launchAtLogin === null}
          onChange={(e) => toggleLaunchAtLogin(e.target.checked)}
        />
        Launch at login
      </label>
      {error && <p className="error">{error}</p>}
    </main>
  );
}
