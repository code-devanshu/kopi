import { useEffect, useState } from "react";
import { commands, unwrap, type PermissionStatus } from "../../lib/ipc";

/** How often the status is checked while something is missing. A grant in System Settings
 * takes effect without a relaunch, so the rows update by themselves. */
const POLL_MS = 2000;

/** The permissions Kopi needs, polled until all are granted. Null until the first answer. */
export function usePermissions(): PermissionStatus | null {
  const [status, setStatus] = useState<PermissionStatus | null>(null);
  const polling = status === null || !allGranted(status);

  useEffect(() => {
    let live = true;
    const check = () =>
      void unwrap(commands.permissionStatus()).then(
        (s) => live && setStatus(s),
        () => {},
      );
    check();
    // Coming back from System Settings.
    window.addEventListener("focus", check);
    const timer = polling ? setInterval(check, POLL_MS) : undefined;
    return () => {
      live = false;
      window.removeEventListener("focus", check);
      clearInterval(timer);
    };
  }, [polling]);

  return status;
}

export function allGranted(status: PermissionStatus): boolean {
  return status.accessibility && status.pasteFromOtherApps === "allowed";
}
