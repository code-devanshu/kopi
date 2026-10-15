import { useEffect, useState } from "react";
import { FormError, Group, Row, Segmented, Switch } from "../../components/Form";
import { PermissionRows } from "../../features/settings/Permissions";
import { ShortcutRecorder } from "../../features/settings/ShortcutRecorder";
import { applyTheme } from "../../features/settings/theme";
import { usePermissions } from "../../features/settings/usePermissions";
import { commands, unwrap, type Theme } from "../../lib/ipc";
import { IS_WINDOWS, TRAY } from "../../lib/platform";

const THEMES: { value: Theme; label: string }[] = [
  { value: "system", label: "System" },
  { value: "light", label: "Light" },
  { value: "dark", label: "Dark" },
];

export function GeneralSection() {
  const [shortcut, setShortcut] = useState<string | null>(null);
  const [theme, setTheme] = useState<Theme>("system");
  const [launch, setLaunch] = useState<boolean | null>(null);
  const [autoUpdate, setAutoUpdate] = useState<boolean | null>(null);
  const [error, setError] = useState<string | null>(null);
  const permissions = usePermissions();

  useEffect(() => {
    unwrap(commands.getSettings()).then(
      (s) => {
        setShortcut(s.shortcut);
        setTheme(s.theme);
        setAutoUpdate(s.autoUpdate);
      },
      (e) => setError(`Couldn't load your settings. ${e}`),
    );
    unwrap(commands.launchAtLogin()).then(setLaunch, () => setLaunch(false));
  }, []);

  function changeTheme(next: Theme) {
    const was = theme;
    setTheme(next);
    applyTheme(next);
    unwrap(commands.setTheme(next)).catch((e) => {
      setTheme(was);
      applyTheme(was);
      setError(`Couldn't change the theme. ${e}`);
    });
  }

  function changeLaunch(next: boolean) {
    setLaunch(next);
    unwrap(commands.setLaunchAtLogin(next)).catch((e) => {
      setLaunch(!next);
      setError(`Couldn't change launch at login. ${e}`);
    });
  }

  function changeAutoUpdate(next: boolean) {
    setAutoUpdate(next);
    unwrap(commands.setAutoUpdate(next)).catch((e) => {
      setAutoUpdate(!next);
      setError(`Couldn't change automatic updates. ${e}`);
    });
  }

  return (
    <>
      <FormError message={error} />
      <Group>
        <Row label="Open Kopi" description="The shortcut that shows Kopi over any app." htmlFor="shortcut">
          {shortcut !== null && <ShortcutRecorder id="shortcut" value={shortcut} onChange={setShortcut} />}
        </Row>
        <Row label="Launch at login" description={`Start Kopi in ${TRAY} when you log in.`} htmlFor="launch">
          <Switch id="launch" checked={launch ?? false} disabled={launch === null} onChange={changeLaunch} />
        </Row>
        <Row label="Appearance">
          <Segmented label="Appearance" value={theme} options={THEMES} onChange={changeTheme} />
        </Row>
        <Row
          label="Update automatically"
          description="Check for updates every few hours and install them. Kopi sends nothing about you or your clips."
          htmlFor="auto-update"
        >
          <Switch
            id="auto-update"
            checked={autoUpdate ?? false}
            disabled={autoUpdate === null}
            onChange={changeAutoUpdate}
          />
        </Row>
      </Group>
      {/* Windows has no permissions to grant. */}
      {!IS_WINDOWS && (
        <Group title="Permissions">
          <PermissionRows status={permissions} />
        </Group>
      )}
    </>
  );
}
