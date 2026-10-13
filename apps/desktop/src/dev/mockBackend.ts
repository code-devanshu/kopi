// A fake backend for working on the panel in a browser (`pnpm dev:ui`). It answers Kopi's
// commands with sample clips, roughly as the Rust side does. Never part of a build.
//
// URL parameters set up a state to look at:
//   ?q=text         types into the search box
//   ?empty          starts with no clips
//   ?select=3       selects the fourth row
//   ?theme=dark     forces dark or light
//   ?preview=closed starts with the preview closed
//   ?paste=pasted   pastes as with Accessibility granted. Without it, a paste only copies, as
//                   when Accessibility isn't granted, which is the case that shows a message.
//   ?paste=error    makes pasting and copying fail
// In the console, kopiDev.copy("text") adds a clip as a copy would.
import { emit } from "@tauri-apps/api/event";
import { mockIPC } from "@tauri-apps/api/mocks";
import { highlight } from "../features/preview/highlight";
import type {
  ClipDetail,
  ClipKind,
  ClipMetadata,
  SearchHit,
  SearchPage,
  SearchQuery,
  SourceApp,
} from "../lib/ipc";

type Clip = ClipDetail;

const APPS: [string, string][] = [
  ["com.apple.Safari", "Safari"],
  ["com.apple.Notes", "Notes"],
  ["com.tinyspeck.slackmacgap", "Slack"],
  ["dev.zed.Zed", "Zed"],
  ["com.apple.Terminal", "Terminal"],
  ["com.apple.mail", "Mail"],
  ["com.figma.Desktop", "Figma"],
  ["com.apple.finder", "Finder"],
];

const SAMPLES: [ClipKind, string | null, ClipMetadata | null, string][] = [
  ["text", "Standup moved to 10:30 tomorrow. Same room, same snacks.", null, "Slack"],
  ["link", "https://github.com/code-devanshu/kopi/pull/29", { host: "github.com" }, "Safari"],
  [
    "code",
    'fn main() {\n    let clips = store.search("invoice")?;\n    println!("{} matches", clips.len());\n}',
    { lang: "rust" },
    "Zed",
  ],
  ["color", "#D08A3A", { rgba: [208, 138, 58, 255] }, "Figma"],
  ["image", null, { w: 1440, h: 900 }, "Safari"],
  [
    "rich_text",
    "Q3 invoice summary\nTotal due: $4,280.00 by 31 October. Please reference INV-2291 on the transfer.",
    null,
    "Mail",
  ],
  ["file", "/Users/me/Desktop/roadmap.pdf\n/Users/me/Desktop/notes.md", { count: 2, names: ["roadmap.pdf", "notes.md"] }, "Finder"],
  ["code", "git switch -c feat/panel-ui && pnpm dev:desktop", { lang: "shell" }, "Terminal"],
  ["text", "The quick brown fox jumps over the lazy dog. ".repeat(12).trim(), null, "Notes"],
  ["link", "https://developer.apple.com/documentation/appkit/nspanel", { host: "developer.apple.com" }, "Safari"],
  ["color", "rgba(30, 144, 255, 0.5)", { rgba: [30, 144, 255, 128] }, "Figma"],
  ["text", "Café au lait, résumé, naïve: accents should match without them.", null, "Notes"],
];

const params = new URLSearchParams(location.search);
const now = Date.now();
let nextId = 1;
let clips: Clip[] = [];

function clip(kind: ClipKind, text: string | null, metadata: ClipMetadata | null, app: string, ago: number): Clip {
  const id = nextId++;
  const appId = APPS.find(([, name]) => name === app)?.[0] ?? null;
  return {
    id,
    kind,
    text,
    contentHash: id.toString(16).padStart(64, "0"),
    sourceAppName: app,
    sourceAppId: appId,
    createdAt: now - ago - 3_600_000,
    lastUsedAt: now - ago,
    useCount: 1 + (id % 4),
    sizeBytes: (text?.length ?? 1_500_000) * (kind === "rich_text" ? 6 : 1),
    pinned: id % 11 === 0,
    metadata,
  };
}

if (!params.has("empty")) {
  // The samples first, then a long tail of notes so paging and scrolling can be tried.
  SAMPLES.forEach(([kind, text, metadata, app], i) => {
    clips.push(clip(kind, text, metadata, app, i * 7 * 60_000 + i * i * 3_600_000));
  });
  for (let i = 0; i < 400; i++) {
    const [, app] = APPS[i % APPS.length];
    clips.push(clip("text", `Note ${i + 1}: ${["meeting", "invoice", "address", "draft", "reply"][i % 5]} ${i * 37}`, null, app, (i + 20) * 5 * 3_600_000));
  }
}

function matches(c: Clip, words: string[]): boolean {
  const hay = `${c.text ?? ""} ${c.sourceAppName ?? ""}`.toLowerCase();
  return words.every((w) => hay.includes(w));
}

function search(query: SearchQuery): SearchPage {
  const words = (query.text ?? "").toLowerCase().split(/\s+/).filter(Boolean);
  const found = clips
    .filter((c) => !query.kind || c.kind === query.kind)
    .filter((c) => !query.sourceAppId || c.sourceAppId === query.sourceAppId)
    .filter((c) => matches(c, words))
    .sort((a, b) => b.lastUsedAt - a.lastUsedAt || b.id - a.id);
  const offset = query.offset ?? 0;
  const limit = Math.min(query.limit || 50, 200);
  const hits: SearchHit[] = found.slice(offset, offset + limit).map((c) => ({
    id: c.id,
    kind: c.kind,
    contentHash: c.contentHash,
    sourceAppName: c.sourceAppName,
    sourceAppId: c.sourceAppId,
    lastUsedAt: c.lastUsedAt,
    pinned: c.pinned,
    metadata: c.metadata,
    snippet: c.text ? highlight(c.text.slice(0, 200), query.text ?? "") : [],
  }));
  return { hits, hasMore: offset + limit < found.length };
}

/** What paste_clip and copy_clip do here: move the clip to the top, as a real paste does. */
function markUsed(id: number): void {
  if (params.get("paste") === "error") throw `mock backend: no clip ${id}`;
  const c = clips.find((c) => c.id === id);
  if (!c) throw `no clip ${id}`;
  c.lastUsedAt = Date.now();
  c.useCount += 1;
  void emit("clip:updated", { id });
}

function sourceApps(): SourceApp[] {
  const seen = new Map<string, SourceApp>();
  for (const c of [...clips].sort((a, b) => b.lastUsedAt - a.lastUsedAt)) {
    if (c.sourceAppId && !seen.has(c.sourceAppId)) {
      seen.set(c.sourceAppId, { id: c.sourceAppId, name: c.sourceAppName });
    }
  }
  return [...seen.values()];
}

mockIPC(
  (cmd, args) => {
    const a = args as Record<string, unknown>;
    switch (cmd) {
      case "search_clips":
        return search(a.query as SearchQuery);
      case "get_clip":
        return clips.find((c) => c.id === a.id) ?? null;
      case "delete_clips": {
        const ids = (a.ids as number[]).filter((id) => clips.some((c) => c.id === id));
        clips = clips.filter((c) => !ids.includes(c.id));
        if (ids.length) void emit("clip:removed", { ids });
        return null;
      }
      case "source_apps":
        return sourceApps();
      case "paste_clip": {
        markUsed(a.id as number);
        const pasted = params.get("paste") === "pasted";
        console.info(`kopi: paste_clip ${a.id}${a.plain ? " as plain text" : ""}`);
        if (pasted) console.info("kopi: hide_panel");
        return pasted ? "pasted" : "copied";
      }
      case "copy_clip":
        markUsed(a.id as number);
        console.info(`kopi: copy_clip ${a.id}${a.plain ? " as plain text" : ""}`);
        return null;
      case "hide_panel":
        console.info("kopi: hide_panel");
        return null;
      case "app_info":
        return { version: "dev", os: "macos" };
    }
    throw `mock backend: no command ${cmd}`;
  },
  { shouldMockEvents: true },
);

const preview = params.get("preview");
if (preview) localStorage.setItem("kopi.preview", preview);

const theme = params.get("theme");
if (theme) document.documentElement.dataset.theme = theme;

// A dark page behind the transparent panel, as on a desktop.
document.documentElement.style.background = "#7a6a5c";
document.body.style.padding = "0";

Object.assign(window, {
  kopiDev: {
    copy(text: string, app = "Notes") {
      const c = clip("text", text, null, app, 0);
      clips.push(c);
      void emit("clip:added", { id: c.id });
    },
  },
});

// Set up the requested state once the panel has loaded.
setTimeout(() => {
  const input = document.querySelector<HTMLInputElement>(".search-input");
  const q = params.get("q");
  if (input && q) {
    const setValue = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!;
    setValue.call(input, q);
    input.dispatchEvent(new Event("input", { bubbles: true }));
  }
  const select = Number(params.get("select") ?? 0);
  for (let i = 0; i < select; i++) {
    input?.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown", bubbles: true }));
  }
}, 300);
