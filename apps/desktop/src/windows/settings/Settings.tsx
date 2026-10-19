import { Clock, RefreshCw, Settings2, ShieldCheck, type LucideIcon } from "lucide-react";
import { useState } from "react";
import { useThemeSetting } from "../../features/settings/theme";
import { CaptureSection } from "./CaptureSection";
import { GeneralSection } from "./GeneralSection";
import { HistorySection } from "./HistorySection";
import { SyncSection } from "./SyncSection";

type Page = "general" | "capture" | "history" | "sync";

const PAGES: { id: Page; label: string; icon: LucideIcon }[] = [
  { id: "general", label: "General", icon: Settings2 },
  { id: "capture", label: "Capture", icon: ShieldCheck },
  { id: "history", label: "History", icon: Clock },
  { id: "sync", label: "Sync", icon: RefreshCw },
];

/** The page on show, remembered while the window is open. */
const PAGE_KEY = "kopi.settings.page";

export function Settings() {
  useThemeSetting();
  const [page, setPage] = useState<Page>(() => {
    const saved = load(PAGE_KEY);
    return PAGES.some((p) => p.id === saved) ? (saved as Page) : "general";
  });

  function show(next: Page) {
    setPage(next);
    save(PAGE_KEY, next);
  }

  return (
    <div className="settings">
      <nav className="settings-nav" aria-label="Settings">
        {PAGES.map(({ id, label, icon: Icon }) => (
          <button
            key={id}
            type="button"
            className="settings-nav-item"
            aria-current={page === id ? "page" : undefined}
            onClick={() => show(id)}
          >
            <Icon size={15} strokeWidth={1.75} aria-hidden="true" />
            {label}
          </button>
        ))}
      </nav>
      <main className="settings-page">
        <h1 className="settings-title">{PAGES.find((p) => p.id === page)!.label}</h1>
        {page === "general" && <GeneralSection />}
        {page === "capture" && <CaptureSection />}
        {page === "history" && <HistorySection />}
        {page === "sync" && <SyncSection />}
      </main>
    </div>
  );
}

// The page is a convenience: without storage, Settings opens on General.
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
