import { X } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { FormError, Group, Row, Select, Switch } from "../../components/Form";
import { KindIcon } from "../../components/KindIcon";
import { KIND_LABELS, KINDS } from "../../lib/format";
import {
  commands,
  unwrap,
  type ClipKind,
  type ExcludedApp,
  type ImageTextStatus,
  type SourceApp,
} from "../../lib/ipc";

/** How often the count of older images left to read is checked while there are some. */
const UNREAD_POLL_MS = 3000;

export function CaptureSection() {
  const [kinds, setKinds] = useState<ClipKind[] | null>(null);
  const [excluded, setExcluded] = useState<ExcludedApp[] | null>(null);
  const [running, setRunning] = useState<SourceApp[]>([]);
  const [seen, setSeen] = useState<SourceApp[]>([]);
  const [imageText, setImageText] = useState<ImageTextStatus | null>(null);
  const [error, setError] = useState<string | null>(null);

  const loadImageText = useCallback(
    () => unwrap(commands.imageTextStatus()).then(setImageText, () => {}),
    [],
  );

  const loadExcluded = useCallback(
    () =>
      unwrap(commands.excludedApps()).then(setExcluded, (e) =>
        setError(`Couldn't load the excluded apps. ${e}`),
      ),
    [],
  );

  useEffect(() => {
    unwrap(commands.getSettings()).then(
      (s) => setKinds(s.captureKinds),
      (e) => setError(`Couldn't load your settings. ${e}`),
    );
    void loadExcluded();
    unwrap(commands.runningApps()).then(setRunning, () => {});
    unwrap(commands.sourceApps()).then(setSeen, () => {});
    void loadImageText();
  }, [loadExcluded, loadImageText]);

  // The backlog's count goes down as it's read.
  const reading = !!imageText?.enabled && imageText.languageInstalled && imageText.unread > 0;
  useEffect(() => {
    if (!reading) return;
    const timer = setInterval(() => void loadImageText(), UNREAD_POLL_MS);
    return () => clearInterval(timer);
  }, [reading, loadImageText]);

  // Coming back from Windows Settings, where a language with text recognition may have been
  // installed.
  const needsLanguage = !!imageText?.available && !imageText.languageInstalled;
  useEffect(() => {
    if (!needsLanguage) return;
    const check = () => void loadImageText();
    window.addEventListener("focus", check);
    return () => window.removeEventListener("focus", check);
  }, [needsLanguage, loadImageText]);

  function toggleImageText(on: boolean) {
    if (!imageText) return;
    const was = imageText;
    setImageText({ ...imageText, enabled: on, unread: on ? imageText.unread : 0 });
    unwrap(commands.setImageText(on)).then(
      () => loadImageText(),
      (e) => {
        setImageText(was);
        setError(`Couldn't change Find text in images. ${e}`);
      },
    );
  }

  function toggleKind(kind: ClipKind, on: boolean) {
    if (!kinds) return;
    const was = kinds;
    const next = KINDS.filter((k) => (k === kind ? on : kinds.includes(k)));
    setKinds(next);
    unwrap(commands.setCaptureKinds(next)).catch((e) => {
      setKinds(was);
      setError(`Couldn't change what Kopi captures. ${e}`);
    });
  }

  function exclude(app: SourceApp) {
    unwrap(commands.addExcludedApp(app.id, app.name ?? app.id)).then(
      () => loadExcluded(),
      (e) => setError(`Couldn't exclude ${app.name ?? app.id}. ${e}`),
    );
  }

  function include(app: ExcludedApp) {
    unwrap(commands.removeExcludedApp(app.id)).then(
      () => loadExcluded(),
      (e) => setError(`Couldn't stop excluding ${app.name}. ${e}`),
    );
  }

  // Apps that could be excluded: open now, then from the history, each once.
  const taken = new Set((excluded ?? []).map((a) => a.id.toLowerCase()));
  const candidates: (SourceApp & { group: string })[] = [];
  for (const [group, apps] of [
    ["Open now", running],
    ["In your history", seen],
  ] as const) {
    for (const app of apps) {
      if (taken.has(app.id.toLowerCase())) continue;
      taken.add(app.id.toLowerCase());
      candidates.push({ ...app, group });
    }
  }

  return (
    <>
      <FormError message={error} />
      <Group
        title="Kinds to capture"
        footer="Copies of a kind that's off aren't saved. When the copy's formats rule out every kind that's on, Kopi doesn't read it at all."
      >
        <div className="kind-grid">
          {KINDS.map((kind) => (
            <label key={kind} className="kind-option">
              <input
                type="checkbox"
                checked={kinds?.includes(kind) ?? false}
                disabled={kinds === null}
                onChange={(e) => toggleKind(kind, e.target.checked)}
              />
              <KindIcon kind={kind} />
              {KIND_LABELS[kind]}
            </label>
          ))}
        </div>
      </Group>
      {imageText?.available && (
        <Group
          footer={reading ? `Reading older images: ${imageText.unread.toLocaleString()} left` : undefined}
        >
          <Row
            label="Find text in images"
            description={
              imageText.languageInstalled ? (
                "Kopi reads the text in images you copy, on this computer, so search can find them. Nothing is sent anywhere."
              ) : (
                <>
                  Add a language with Optical character recognition in Windows Settings → Time &amp; language.{" "}
                  <button type="button" className="link-button" onClick={() => void commands.openLanguageSettings()}>
                    Open Language Settings
                  </button>
                </>
              )
            }
            htmlFor="image-text"
          >
            <Switch
              id="image-text"
              checked={imageText.enabled && imageText.languageInstalled}
              disabled={!imageText.languageInstalled}
              onChange={toggleImageText}
            />
          </Row>
        </Group>
      )}
      <Group
        title="Excluded apps"
        footer="Kopi never reads copies made in these apps. Copies an app marks as a password are skipped too, whichever app they come from."
      >
        {excluded?.length === 0 && <p className="group-empty muted">No apps are excluded.</p>}
        {excluded?.map((app) => (
          <div key={app.id} className="app-row">
            <div className="app-row-text">
              <span className="app-row-name">
                {app.name}
                {app.isDefault && <span className="tag">Built in</span>}
              </span>
              <span className="app-row-id mono muted">{app.id}</span>
            </div>
            <button
              type="button"
              className="icon-button"
              aria-label={`Stop excluding ${app.name}`}
              title="Stop excluding"
              onClick={() => include(app)}
            >
              <X size={14} strokeWidth={2} aria-hidden="true" />
            </button>
          </div>
        ))}
        <div className="app-row add">
          <Select
            label="Exclude an app"
            value=""
            options={[
              { value: "", label: candidates.length ? "Exclude an app…" : "No other apps to exclude" },
              ...candidates.map((a) => ({ value: a.id, label: a.name ?? a.id, group: a.group })),
            ]}
            onChange={(id) => {
              const app = candidates.find((a) => a.id === id);
              if (app) exclude(app);
            }}
          />
        </div>
      </Group>
    </>
  );
}
