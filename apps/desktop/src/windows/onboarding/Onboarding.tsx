import { HardDrive, Keyboard, ShieldCheck, type LucideIcon } from "lucide-react";
import { useEffect, useRef, useState, type ReactNode, type Ref } from "react";
import icon from "../../../src-tauri/icons/128x128@2x.png";
import { FormError, Group, Row, Switch } from "../../components/Form";
import { PermissionRows } from "../../features/settings/Permissions";
import { clash, DEFAULT_SHORTCUT, parts } from "../../features/settings/shortcut";
import { ShortcutRecorder } from "../../features/settings/ShortcutRecorder";
import { useThemeSetting } from "../../features/settings/theme";
import { usePermissions } from "../../features/settings/usePermissions";
import { commands, unwrap, type Permission, type PermissionStatus } from "../../lib/ipc";
import { DEVICE, TRAY } from "../../lib/platform";

type Step = "welcome" | "permissions" | "shortcut" | "done";

const STEPS: Step[] = ["welcome", "permissions", "shortcut", "done"];

/** The step on show, remembered while the window is open. */
const STEP_KEY = "kopi.onboarding.step";

const PERMISSION_NAMES: Record<Permission, string> = {
  accessibility: "Accessibility",
  pasteFromOtherApps: "Paste from Other Apps",
};

export function Onboarding() {
  useThemeSetting();
  const status = usePermissions();
  // What to ask for, fixed by the first status, so a row stays once it's granted. The
  // permissions step is skipped if nothing is missing, as on macOS versions that don't ask for
  // Paste from Other Apps once Accessibility is allowed.
  const [needed, setNeeded] = useState<Permission[] | null>(null);
  if (status && needed === null) setNeeded(missing(status));

  const [step, setStep] = useState<Step>(() => {
    const saved = load(STEP_KEY);
    return STEPS.find((s) => s === saved) ?? "welcome";
  });
  const [shortcut, setShortcut] = useState<string | null>(null);
  const [launch, setLaunch] = useState<boolean | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [finishing, setFinishing] = useState(false);
  const title = useRef<HTMLHeadingElement>(null);
  const moved = useRef(false);

  useEffect(() => {
    unwrap(commands.getSettings()).then(
      (s) => setShortcut(s.shortcut),
      () => setShortcut(DEFAULT_SHORTCUT),
    );
    unwrap(commands.launchAtLogin()).then(setLaunch, () => setLaunch(false));
  }, []);

  // Moves focus to the new step's title, so a screen reader reads it.
  useEffect(() => {
    if (moved.current) title.current?.focus();
  }, [step]);

  const steps = needed?.length === 0 ? STEPS.filter((s) => s !== "permissions") : STEPS;
  const current = steps.includes(step) ? step : "shortcut";
  const index = steps.indexOf(current);

  function go(next: Step) {
    moved.current = true;
    setStep(next);
    save(STEP_KEY, next);
  }

  function changeLaunch(next: boolean) {
    setLaunch(next);
    unwrap(commands.setLaunchAtLogin(next)).catch((e) => {
      setLaunch(!next);
      setError(`Couldn't change launch at login. ${e}`);
    });
  }

  async function finish(openPanel: boolean) {
    setFinishing(true);
    try {
      // Closes the window.
      await unwrap(commands.finishOnboarding(openPanel));
    } catch (e) {
      setError(`Couldn't finish. ${e}`);
      setFinishing(false);
    }
  }

  const keys = <Keys shortcut={shortcut} />;
  const stillMissing = status ? missing(status) : [];
  const granted = needed !== null && needed.every((p) => !stillMissing.includes(p));

  let body: ReactNode;
  let primary: ReactNode;
  if (current === "welcome") {
    body = (
      <>
        <img className="onboarding-icon" src={icon} alt="" width={64} height={64} />
        <Title ref={title}>Welcome to Kopi</Title>
        <p className="onboarding-lead">Kopi keeps what you copy, so you can find it and paste it again.</p>
        <ul className="onboarding-points">
          <Point icon={Keyboard}>Press {keys} in any app to search what you've copied and paste it.</Point>
          <Point icon={HardDrive}>Your clips stay on {DEVICE}. Kopi doesn't send them anywhere.</Point>
          <Point icon={ShieldCheck}>
            Kopi skips password managers and copies that apps mark as private. You can add apps to skip in Settings.
          </Point>
        </ul>
        <button type="button" className="link-button onboarding-link" onClick={() => void commands.openPrivacyDesign()}>
          How Kopi handles your clips
        </button>
      </>
    );
    primary = (
      <button type="button" className="button primary" autoFocus onClick={() => go(steps[index + 1])}>
        Get Started
      </button>
    );
  } else if (current === "permissions") {
    body = (
      <>
        <Title ref={title}>Allow Kopi in System Settings</Title>
        <p className="onboarding-lead">
          macOS asks you to allow these yourself. Each one shows as allowed here as soon as it is.
        </p>
        <Group footer="You can skip this and allow them later in Settings.">
          <PermissionRows status={status} only={needed ?? undefined} ask />
        </Group>
      </>
    );
    primary = (
      <button type="button" className="button primary" onClick={() => go(steps[index + 1])}>
        {granted ? "Continue" : "Not Now"}
      </button>
    );
  } else if (current === "shortcut") {
    body = (
      <>
        <Title ref={title}>Open Kopi from anywhere</Title>
        <p className="onboarding-lead">Press the shortcut in any app to show your clips. Click it to choose another.</p>
        <FormError message={error} />
        <Group>
          <Row label="Shortcut" description={(shortcut !== null && clash(shortcut)) || undefined} htmlFor="shortcut">
            {shortcut !== null && <ShortcutRecorder id="shortcut" value={shortcut} onChange={setShortcut} />}
          </Row>
          <Row label="Launch at login" description={`Start Kopi in ${TRAY} when you log in.`} htmlFor="launch">
            <Switch id="launch" checked={launch ?? false} disabled={launch === null} onChange={changeLaunch} />
          </Row>
        </Group>
      </>
    );
    primary = (
      <button type="button" className="button primary" onClick={() => go(steps[index + 1])}>
        Continue
      </button>
    );
  } else {
    body = (
      <>
        <Title ref={title}>{stillMissing.length ? "Almost done" : "You're all set"}</Title>
        <p className="onboarding-lead">Copy something, then press {keys} to find it again.</p>
        <p className="onboarding-text">
          Kopi lives in {TRAY}. Its menu has Settings, and Set Up Kopi brings you back here.
        </p>
        {stillMissing.length > 0 && (
          <p className="onboarding-note">
            Kopi still needs {stillMissing.map((p) => PERMISSION_NAMES[p]).join(" and ")}. You can allow{" "}
            {stillMissing.length === 1 ? "it" : "them"} any time in Settings.
          </p>
        )}
        <FormError message={error} />
      </>
    );
    primary = (
      <>
        <button type="button" className="button" disabled={finishing} onClick={() => void finish(false)}>
          Done
        </button>
        <button type="button" className="button primary" disabled={finishing} onClick={() => void finish(true)}>
          Open Kopi
        </button>
      </>
    );
  }

  return (
    <div className="onboarding">
      <main className="onboarding-step">{body}</main>
      <footer className="onboarding-footer">
        <div className="onboarding-progress" role="img" aria-label={`Step ${index + 1} of ${steps.length}`}>
          {steps.map((s) => (
            <span key={s} className={s === current ? "current" : undefined} />
          ))}
        </div>
        <div className="button-row">
          {index > 0 && (
            <button type="button" className="button" onClick={() => go(steps[index - 1])}>
              Back
            </button>
          )}
          {primary}
        </div>
      </footer>
    </div>
  );
}

function missing(status: PermissionStatus): Permission[] {
  const out: Permission[] = [];
  if (!status.accessibility) out.push("accessibility");
  if (status.pasteFromOtherApps !== "allowed") out.push("pasteFromOtherApps");
  return out;
}

function Title({ ref, children }: { ref: Ref<HTMLHeadingElement>; children: ReactNode }) {
  return (
    <h1 ref={ref} className="onboarding-title" tabIndex={-1}>
      {children}
    </h1>
  );
}

function Point({ icon: Icon, children }: { icon: LucideIcon; children: ReactNode }) {
  return (
    <li className="onboarding-point">
      <span className="onboarding-point-icon">
        <Icon size={15} strokeWidth={1.75} aria-hidden="true" />
      </span>
      <p>{children}</p>
    </li>
  );
}

/** A shortcut as keycaps: ⇧ ⌘ V. */
function Keys({ shortcut }: { shortcut: string | null }) {
  const keys = parts(shortcut ?? DEFAULT_SHORTCUT);
  return (
    <span className="keys">
      {keys.map((k, i) => (
        <kbd key={i}>{k}</kbd>
      ))}
    </span>
  );
}

// The step is a convenience: without storage, onboarding opens on Welcome.
function load(key: string): string | null {
  try {
    return sessionStorage.getItem(key);
  } catch {
    return null;
  }
}

function save(key: string, value: string) {
  try {
    sessionStorage.setItem(key, value);
  } catch {
    // Not saved. It's only a convenience.
  }
}
