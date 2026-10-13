import { CircleAlert, CircleCheck } from "lucide-react";
import { useState } from "react";
import { Row } from "../../components/Form";
import { commands, type PasteAccess, type Permission, type PermissionStatus } from "../../lib/ipc";

const PASTE_STATUS: Record<PasteAccess, string> = {
  allowed: "Allowed",
  notAsked: "Not set up",
  ask: "Asks for each copy",
  denied: "Denied",
};

/**
 * A row per permission, with its status and a way to grant it. `only` picks the rows to show.
 * With `ask`, the Accessibility button shows the system's request first: macOS lists Kopi under
 * Accessibility only once it has asked.
 */
export function PermissionRows({
  status,
  only,
  ask = false,
}: {
  status: PermissionStatus | null;
  only?: Permission[];
  ask?: boolean;
}) {
  const shows = (permission: Permission) => !only || only.includes(permission);
  return (
    <>
      {shows("accessibility") && (
        <Row
          label={
            <>
              Accessibility
              <PermissionState
                granted={status?.accessibility}
                label={status?.accessibility ? "Allowed" : "Not allowed"}
              />
            </>
          }
          description="Lets Kopi paste into the app you're using. Without it, Kopi copies the clip and you press ⌘V."
        >
          {status &&
            !status.accessibility &&
            (ask ? <AskAccessibility /> : <OpenSettings permission="accessibility" />)}
        </Row>
      )}
      {shows("pasteFromOtherApps") && (
        <Row
          label={
            <>
              Paste from Other Apps
              <PermissionState
                granted={status ? status.pasteFromOtherApps === "allowed" : undefined}
                label={status ? PASTE_STATUS[status.pasteFromOtherApps] : ""}
              />
            </>
          }
          description="Lets Kopi see what you copy. In Privacy & Security, set Kopi to Allow."
        >
          {status && status.pasteFromOtherApps !== "allowed" && <OpenSettings permission="pasteFromOtherApps" />}
        </Row>
      )}
    </>
  );
}

function PermissionState({ granted, label }: { granted: boolean | undefined; label: string }) {
  if (granted === undefined) return <span className="permission-state muted">Checking…</span>;
  const Icon = granted ? CircleCheck : CircleAlert;
  return (
    <span className={granted ? "permission-state granted" : "permission-state missing"}>
      <Icon size={14} strokeWidth={2} aria-hidden="true" />
      {label}
    </span>
  );
}

/** Asks macOS first. Its alert offers to open System Settings, so after that the button does. */
function AskAccessibility() {
  const [asked, setAsked] = useState(false);
  if (asked) return <OpenSettings permission="accessibility" />;
  return (
    <button
      type="button"
      className="button primary"
      onClick={() => {
        setAsked(true);
        void commands.requestPermission("accessibility");
      }}
    >
      Allow…
    </button>
  );
}

function OpenSettings({ permission }: { permission: Permission }) {
  return (
    <button type="button" className="button" onClick={() => void commands.openPermissionSettings(permission)}>
      Open System Settings
    </button>
  );
}
