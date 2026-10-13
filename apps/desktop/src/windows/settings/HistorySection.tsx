import { useEffect, useState } from "react";
import { FormError, Group, Row, Select } from "../../components/Form";
import {
  AGE_CHOICES,
  ageLabel,
  fromOption,
  ITEM_CHOICES,
  itemLabel,
  SIZE_CHOICES,
  sizeLabel,
  toOption,
  withCurrent,
  type Choice,
} from "../../features/settings/limits";
import { commands, unwrap, type ClearRange, type HistoryLimits } from "../../lib/ipc";

const RANGE_LABELS: Record<ClearRange, string> = {
  lastHour: "the last hour",
  lastDay: "the last day",
  all: "your history",
};

export function HistorySection() {
  const [limits, setLimits] = useState<HistoryLimits | null>(null);
  const [keepPinned, setKeepPinned] = useState(true);
  const [confirming, setConfirming] = useState(false);
  const [clearing, setClearing] = useState(false);
  const [cleared, setCleared] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    unwrap(commands.historyLimits()).then(setLimits, (e) => setError(`Couldn't load the history limits. ${e}`));
  }, []);

  function change(key: keyof HistoryLimits, value: number | null) {
    if (!limits) return;
    const was = limits;
    const next = { ...limits, [key]: value };
    setLimits(next);
    unwrap(commands.setHistoryLimits(next)).catch((e) => {
      setLimits(was);
      setError(`Couldn't change the limit. ${e}`);
    });
  }

  async function clear(range: ClearRange) {
    setConfirming(false);
    setClearing(true);
    setCleared(null);
    try {
      const count = await unwrap(commands.clearHistory(range, keepPinned));
      setCleared(count === 0 ? "Nothing to delete." : `Deleted ${count === 1 ? "1 clip" : `${count.toLocaleString()} clips`}.`);
    } catch (e) {
      setError(`Couldn't clear ${RANGE_LABELS[range]}. ${e}`);
    } finally {
      setClearing(false);
    }
  }

  const limitRow = (
    key: keyof HistoryLimits,
    label: string,
    description: string,
    choices: Choice[],
    format: (n: number) => string,
  ) => {
    const value = limits?.[key] ?? null;
    return (
      <Row label={label} description={description} htmlFor={key}>
        {limits && (
          <Select
            id={key}
            value={toOption(value)}
            options={withCurrent(choices, value, format).map((c) => ({ value: toOption(c.value), label: c.label }))}
            onChange={(option) => change(key, fromOption(option))}
          />
        )}
      </Row>
    );
  };

  return (
    <>
      <FormError message={error} />
      <Group title="Keep" footer="Pinned clips and clips on a pinboard are never removed by these limits.">
        {limitRow("maxItems", "Clips", "The most clips to keep. The oldest go first.", ITEM_CHOICES, itemLabel)}
        {limitRow("maxAgeDays", "Remove after", "Clips you haven't used for this long are removed.", AGE_CHOICES, ageLabel)}
        {limitRow(
          "maxTotalBytes",
          "Storage",
          "The most space your history may take, pinned clips included.",
          SIZE_CHOICES,
          sizeLabel,
        )}
      </Group>
      <Group
        title="Clear history"
        footer="Deleted clips are overwritten in Kopi's database. Copies can still exist in backups, such as Time Machine."
      >
        <Row label="Keep pinned clips and pinboards" htmlFor="keep-pinned">
          <input
            id="keep-pinned"
            type="checkbox"
            className="checkbox"
            checked={keepPinned}
            onChange={(e) => setKeepPinned(e.target.checked)}
          />
        </Row>
        <Row label="Clear" description={cleared ?? undefined}>
          <div className="button-row">
            <button type="button" className="button" disabled={clearing} onClick={() => void clear("lastHour")}>
              Last hour
            </button>
            <button type="button" className="button" disabled={clearing} onClick={() => void clear("lastDay")}>
              Last day
            </button>
            <button
              type="button"
              className="button danger"
              disabled={clearing || confirming}
              onClick={() => setConfirming(true)}
            >
              Everything…
            </button>
          </div>
        </Row>
        {confirming && (
          <div className="confirm" role="alertdialog" aria-labelledby="confirm-text">
            <p id="confirm-text">
              {keepPinned
                ? "Delete every clip except pinned clips and those on a pinboard?"
                : "Delete every clip, pinned clips and pinboard contents included?"}{" "}
              This can't be undone.
            </p>
            <div className="button-row">
              <button type="button" className="button" autoFocus onClick={() => setConfirming(false)}>
                Cancel
              </button>
              <button type="button" className="button danger solid" onClick={() => void clear("all")}>
                Delete everything
              </button>
            </div>
          </div>
        )}
      </Group>
    </>
  );
}
