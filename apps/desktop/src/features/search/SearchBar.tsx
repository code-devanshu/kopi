import { ChevronDown, Search } from "lucide-react";
import type { RefObject } from "react";
import { KIND_LABELS, KINDS } from "../../lib/format";
import type { ClipKind, SourceApp } from "../../lib/ipc";

type Props = {
  inputRef: RefObject<HTMLInputElement | null>;
  text: string;
  kind: ClipKind | null;
  appId: string | null;
  apps: SourceApp[];
  listId: string;
  activeId: string | undefined;
  onText: (text: string) => void;
  onKind: (kind: ClipKind | null) => void;
  onApp: (appId: string | null) => void;
};

export function SearchBar({
  inputRef,
  text,
  kind,
  appId,
  apps,
  listId,
  activeId,
  onText,
  onKind,
  onApp,
}: Props) {
  // Keep a filtered app listed even once its clips are gone, so the filter can be cleared.
  const listed =
    appId && !apps.some((a) => a.id === appId) ? [...apps, { id: appId, name: null }] : apps;
  // The search box gets focus back once a filter is picked, so typing carries on.
  const refocus = () => inputRef.current?.focus();

  return (
    <div className="search">
      <Search className="search-icon" size={16} strokeWidth={1.75} aria-hidden="true" />
      <input
        ref={inputRef}
        className="search-input"
        type="text"
        role="combobox"
        aria-label="Search clips"
        aria-expanded="true"
        aria-controls={listId}
        aria-activedescendant={activeId}
        placeholder="Search clips"
        value={text}
        onChange={(e) => onText(e.target.value)}
        autoFocus
        spellCheck={false}
        autoCorrect="off"
        autoCapitalize="off"
        autoComplete="off"
      />
      <Filter
        label="Kind"
        value={kind ?? ""}
        onChange={(v) => {
          onKind((v || null) as ClipKind | null);
          refocus();
        }}
      >
        <option value="">All kinds</option>
        {KINDS.map((k) => (
          <option key={k} value={k}>
            {KIND_LABELS[k]}
          </option>
        ))}
      </Filter>
      <Filter
        label="App"
        value={appId ?? ""}
        onChange={(v) => {
          onApp(v || null);
          refocus();
        }}
      >
        <option value="">All apps</option>
        {listed.map((app) => (
          <option key={app.id} value={app.id}>
            {app.name ?? app.id}
          </option>
        ))}
      </Filter>
    </div>
  );
}

function Filter({
  label,
  value,
  onChange,
  children,
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
  children: React.ReactNode;
}) {
  return (
    <label className={value ? "filter active" : "filter"}>
      <span className="visually-hidden">{label}</span>
      <select value={value} onChange={(e) => onChange(e.target.value)}>
        {children}
      </select>
      <ChevronDown className="filter-chevron" size={13} strokeWidth={2} aria-hidden="true" />
    </label>
  );
}
