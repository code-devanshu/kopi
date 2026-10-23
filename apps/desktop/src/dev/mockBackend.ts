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
//   ?noboards       starts with no pinboards
//   ?nosnippets     starts with no snippets
//   ?granted        Settings shows every permission granted. Without it, neither is.
//   ?granted=some   Accessibility is granted, and Paste from Other Apps isn't
//   ?shortcut=taken makes recording a shortcut fail, as when another app holds it
//   ?page=history   opens Settings on that page
//   ?step=done      opens onboarding on that step
//   ?sync           starts with sync on and two paired computers. ?sync=denied adds the
//                   Local Network problem. Pairing works with the code 482913.
//   ?tab=Snippets   opens that tab of the panel: Pinned, Snippets or a pinboard's name
//   ?shot           leaves the page behind the panel transparent, for the website's screenshots
//   ?reading        the sample image's text hasn't been read yet, and Settings shows a backlog
//   ?nolanguage     Settings → Capture shows Windows without a language for text recognition
// Open /settings.html for the Settings window, and /onboarding.html for onboarding.
// In the console, kopiDev.copy("text") adds a clip as a copy would.
import { emit } from "@tauri-apps/api/event";
import { mockIPC } from "@tauri-apps/api/mocks";
import { highlight } from "../features/preview/highlight";
import type {
  ClearRange,
  ClipDetail,
  ClipKind,
  ClipMetadata,
  ExcludedApp,
  HistoryLimits,
  Pinboard,
  SearchHit,
  SearchPage,
  SearchQuery,
  Settings,
  Snippet,
  SnippetFields,
  SourceApp,
  SyncStatus,
  Theme,
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
  // The text read from the screenshot.
  [
    "image",
    "Build failed\nerror[E0425]: cannot find value `clips` in this scope\n--> src/main.rs:2:9",
    { w: 1440, h: 900 },
    "Safari",
  ],
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
    copiedOn: id % 5 === 0 ? "Windows PC" : null,
    readingText: false,
  };
}

// The long tail reads like real copies, so screenshots of a search look real too.
const PEOPLE = ["Priya", "Sam", "Lena", "Marco", "Aiko", "Tom"];
const CLIENTS = ["Northwind", "Acme Studio", "Blue Fern", "Halcyon Labs", "Oakline"];
function tail(i: number): string {
  // Each of the five kinds of note comes round every fifth clip, so these count rounds.
  const round = Math.floor(i / 5);
  const person = PEOPLE[round % PEOPLE.length];
  const client = CLIENTS[round % CLIENTS.length];
  switch (i % 5) {
    case 0:
      return `Meeting with ${client} moved to ${9 + (i % 8)}:30, link in the invite`;
    case 1:
      return `Invoice INV-${2300 + i} for ${client}, due in ${[7, 14, 30][round % 3]} days`;
    case 2:
      return `${12 + (i % 80)} Harbour Street, Unit ${1 + (i % 9)}, Bristol BS1 ${4 + (i % 5)}QA`;
    case 3:
      return `Draft: thanks ${person}, I'll send the revised plan by Friday`;
    default:
      return `Re: ${client} launch. Sounds good, ${person}, let's go with option B`;
  }
}

if (!params.has("empty")) {
  // The samples first, then a long tail of notes so paging and scrolling can be tried.
  SAMPLES.forEach(([kind, text, metadata, app], i) => {
    clips.push(clip(kind, text, metadata, app, i * 7 * 60_000 + i * i * 3_600_000));
  });
  if (params.has("reading")) {
    for (const c of clips) if (c.kind === "image") Object.assign(c, { text: null, readingText: true });
  }
  for (let i = 0; i < 400; i++) {
    const [, app] = APPS[i % APPS.length];
    clips.push(clip("text", tail(i), null, app, (i + 20) * 5 * 3_600_000));
  }
}

// Snippets, in the user's order.
let nextSnippetId = 1;
let snippets: Snippet[] = [];

function addSnippet(fields: SnippetFields): Snippet {
  const name = fields.name.trim();
  if (!name) throw "a snippet needs a name";
  if (fields.keyword && /\s/.test(fields.keyword)) throw "a keyword can't have spaces";
  const at = Date.now();
  const s: Snippet = {
    id: nextSnippetId++,
    name,
    keyword: fields.keyword?.trim() || null,
    body: fields.body,
    createdAt: at,
    updatedAt: at,
    lastUsedAt: null,
    useCount: 0,
  };
  snippets.unshift(s);
  return s;
}

function snippet(id: number): Snippet {
  const s = snippets.find((s) => s.id === id);
  if (!s) throw "That snippet was deleted.";
  return s;
}

function findSnippets(text: string): Snippet[] {
  const exact = (s: Snippet) => !!text.trim() && s.keyword?.toLowerCase() === text.trim().toLowerCase();
  const words = text.toLowerCase().split(/\s+/).filter(Boolean);
  const has = (s: Snippet) => words.every((w) => `${s.name}\n${s.keyword ?? ""}\n${s.body}`.toLowerCase().includes(w));
  return [...snippets.filter(exact), ...snippets.filter((s) => !exact(s) && has(s))];
}

function markSnippetUsed(id: number, text: string) {
  const s = snippet(id);
  s.useCount++;
  s.lastUsedAt = Date.now();
  console.info(`kopi: snippet ${id}:`, JSON.stringify(text));
}

if (!params.has("nosnippets")) {
  for (const fields of [
    { name: "Thanks for your order", keyword: "order", body: "Hi {input:Name},\n\nThanks for your order on {date}. Your tracking link is {clipboard}\n\nDevanshu" },
    { name: "Signature", keyword: "sig", body: "Thanks,\nDevanshu Verma\nkopi.devanshuverma.in" },
    { name: "Bold tag", keyword: null, body: "<b>{cursor}</b>" },
    { name: "Changelog heading", keyword: "cl", body: "## {date:YYYY-MM-DD}\n\n- {cursor}" },
  ].reverse()) {
    addSnippet(fields);
  }
}

// Pinboards, and the clips on each, top first.
let nextBoardId = 1;
let boards: Pinboard[] = [];
const items = new Map<number, number[]>();

function addBoard(name: string, color: string | null): Pinboard {
  const trimmed = name.trim();
  if (!trimmed) throw "a pinboard needs a name";
  const board = { id: nextBoardId++, name: trimmed, color, clipCount: 0 };
  boards.push(board);
  items.set(board.id, []);
  return board;
}

function boardList(): Pinboard[] {
  return boards.map((b) => ({ ...b, clipCount: items.get(b.id)?.length ?? 0 }));
}

function board(id: number): Pinboard {
  const b = boards.find((b) => b.id === id);
  if (!b) throw "That pinboard was deleted.";
  return b;
}

if (!params.has("empty") && !params.has("noboards")) {
  items.set(addBoard("Work", "#D08A3A").id, [2, 6, 8]);
  items.set(addBoard("Snippets", "#4A78B5").id, [3]);
  items.set(addBoard("Colors", "#8A63A8").id, [4, 11]);
}

function matches(c: Clip, words: string[]): boolean {
  const hay = `${c.text ?? ""} ${c.sourceAppName ?? ""}`.toLowerCase();
  return words.every((w) => hay.includes(w));
}

/** The row's text. An image shows the text read from it only where the search matched it. */
function snippetOf(c: Clip, text: string) {
  if (!c.text) return [];
  const spans = highlight(c.text.slice(0, 200), text);
  return c.kind === "image" && !spans.some((s) => s.matched) ? [] : spans;
}

function search(query: SearchQuery): SearchPage {
  const words = (query.text ?? "").toLowerCase().split(/\s+/).filter(Boolean);
  const onBoard = query.pinboardId != null ? (items.get(query.pinboardId) ?? []) : null;
  const found = clips
    .filter((c) => !query.kind || c.kind === query.kind)
    .filter((c) => !query.sourceAppId || c.sourceAppId === query.sourceAppId)
    .filter((c) => !query.pinned || c.pinned)
    .filter((c) => !onBoard || onBoard.includes(c.id))
    .filter((c) => matches(c, words))
    .sort((a, b) =>
      // A pinboard lists in its own order until there's text to rank.
      onBoard && !words.length
        ? onBoard.indexOf(a.id) - onBoard.indexOf(b.id)
        : b.lastUsedAt - a.lastUsedAt || b.id - a.id,
    );
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
    snippet: snippetOf(c, query.text ?? ""),
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

// Settings.
const settings: Settings = {
  shortcut: "shift+super+KeyV",
  captureKinds: ["text", "rich_text", "link", "image", "file", "color", "code"],
  theme: (params.get("theme") as Theme | null) ?? "system",
  autoUpdate: true,
};
let limits: HistoryLimits = { maxItems: null, maxAgeDays: null, maxTotalBytes: 2 * 1024 ** 3 };
let launchAtLogin = false;
let imageText = { enabled: true, unread: params.has("reading") ? 1240 : 0 };
let excluded: ExcludedApp[] = [
  { id: "com.1password.1password", name: "1Password", isDefault: true },
  { id: "com.apple.keychainaccess", name: "Keychain Access", isDefault: true },
  { id: "com.apple.Passwords", name: "Passwords", isDefault: true },
];
const granted = params.get("granted");

function clearHistory(range: ClearRange, keepPinned: boolean): number {
  const hour = 3_600_000;
  const since = range === "all" ? -Infinity : Date.now() - (range === "lastHour" ? hour : 24 * hour);
  const kept = new Set([...items.values()].flat());
  const ids = clips
    .filter((c) => c.lastUsedAt >= since && !(keepPinned && (c.pinned || kept.has(c.id))))
    .map((c) => c.id);
  clips = clips.filter((c) => !ids.includes(c.id));
  if (ids.length) void emit("clip:removed", { ids });
  return ids.length;
}

const syncParam = params.get("sync");
const sync: SyncStatus = {
  enabled: syncParam !== null,
  deviceName: "MacBook Air",
  kinds: ["text", "rich_text", "link", "code", "color", "image"],
  snippets: true,
  devices:
    syncParam === null
      ? []
      : [
          {
            id: "pc",
            name: "Windows PC",
            os: "windows",
            connected: true,
            needsUpdate: false,
            lastSeenAt: now,
            stale: false,
          },
          {
            id: "mini",
            name: "Mac mini",
            os: "macos",
            connected: false,
            needsUpdate: false,
            lastSeenAt: now - 2 * 3_600_000,
            stale: false,
          },
        ],
  problem: syncParam === "denied" ? "localNetworkDenied" : null,
  pairing: null,
};

function syncChanged() {
  void emit("sync:changed", null);
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
        for (const [board, on] of items) items.set(board, on.filter((id) => !ids.includes(id)));
        if (ids.length) void emit("clip:removed", { ids });
        return null;
      }
      case "set_pinned": {
        const c = clips.find((c) => c.id === a.id);
        if (c && c.pinned !== a.pinned) {
          c.pinned = a.pinned as boolean;
          void emit("clip:updated", { id: c.id });
        }
        return null;
      }
      case "pinboard_list":
        return boardList();
      case "pinboard_create": {
        const b = addBoard(a.name as string, a.color as string | null);
        void emit("pinboards:changed");
        return b;
      }
      case "pinboard_rename": {
        const name = (a.name as string).trim();
        if (!name) throw "a pinboard needs a name";
        board(a.id as number).name = name;
        void emit("pinboards:changed");
        return null;
      }
      case "pinboard_recolor":
        board(a.id as number).color = a.color as string | null;
        void emit("pinboards:changed");
        return null;
      case "pinboard_reorder": {
        const ids = a.ids as number[];
        boards.sort((x, y) => {
          const i = ids.indexOf(x.id);
          const j = ids.indexOf(y.id);
          return (i < 0 ? Infinity : i) - (j < 0 ? Infinity : j);
        });
        void emit("pinboards:changed");
        return null;
      }
      case "pinboard_delete":
        boards = boards.filter((b) => b.id !== a.id);
        items.delete(a.id as number);
        void emit("pinboards:changed");
        return null;
      case "pinboard_add_clips": {
        board(a.id as number);
        const on = items.get(a.id as number)!;
        const add = (a.clipIds as number[]).filter((id) => !on.includes(id));
        items.set(a.id as number, [...add, ...on]);
        if (add.length) void emit("pinboards:changed");
        return null;
      }
      case "pinboard_remove_clips": {
        const on = items.get(a.id as number) ?? [];
        const left = on.filter((id) => !(a.clipIds as number[]).includes(id));
        items.set(a.id as number, left);
        if (left.length !== on.length) void emit("pinboards:changed");
        return null;
      }
      case "clip_pinboards":
        return boards.filter((b) => items.get(b.id)?.includes(a.id as number)).map((b) => b.id);
      case "source_apps":
        return sourceApps();
      case "snippet_search":
        return findSnippets(a.text as string);
      case "snippet_keyword":
        return a.text && (a.text as string).trim() ? findSnippets(a.text as string).filter((s) => s.keyword?.toLowerCase() === (a.text as string).trim().toLowerCase()) : [];
      case "snippet_get":
        return snippets.find((s) => s.id === a.id) ?? null;
      case "snippet_create": {
        const s = addSnippet(a.fields as SnippetFields);
        void emit("snippets:changed");
        return s;
      }
      case "snippet_update": {
        const s = snippet(a.id as number);
        const fields = a.fields as SnippetFields;
        if (!fields.name.trim()) throw "a snippet needs a name";
        Object.assign(s, { name: fields.name.trim(), keyword: fields.keyword?.trim() || null, body: fields.body, updatedAt: Date.now() });
        void emit("snippets:changed");
        return null;
      }
      case "snippet_reorder": {
        const ids = a.ids as number[];
        snippets.sort((x, y) => {
          const i = ids.indexOf(x.id);
          const j = ids.indexOf(y.id);
          return (i < 0 ? Infinity : i) - (j < 0 ? Infinity : j);
        });
        void emit("snippets:changed");
        return null;
      }
      case "snippet_delete":
        snippets = snippets.filter((s) => s.id !== a.id);
        void emit("snippets:changed");
        return null;
      case "paste_snippet": {
        if (params.get("paste") === "error") throw "the pasteboard is busy";
        markSnippetUsed(a.id as number, a.text as string);
        const pasted = params.get("paste") === "pasted";
        if (pasted) console.info(`kopi: hide_panel, then ${a.cursorBack} × ←`);
        return pasted ? "pasted" : "copied";
      }
      case "copy_snippet":
        if (params.get("paste") === "error") throw "the pasteboard is busy";
        markSnippetUsed(a.id as number, a.text as string);
        return null;
      case "clipboard_text":
        return [...clips].filter((c) => c.text && ["text", "rich_text", "link", "code", "color"].includes(c.kind)).sort((x, y) => y.lastUsedAt - x.lastUsedAt)[0]?.text ?? null;
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
      case "copy_image_text":
        console.info(`kopi: copy_image_text ${a.id}`);
        return null;
      case "image_text_status":
        return { available: true, languageInstalled: !params.has("nolanguage"), ...imageText };
      case "set_image_text":
        imageText = { enabled: a.enabled as boolean, unread: a.enabled ? 1240 : 0 };
        for (const c of clips) if (c.kind === "image") c.readingText = imageText.enabled;
        return null;
      case "hide_panel":
        console.info("kopi: hide_panel");
        return null;
      case "app_info":
        return { version: "dev", os: "macos" };
      case "get_settings":
        return settings;
      case "set_shortcut":
        if (params.get("shortcut") === "taken") throw `${a.shortcut} is taken by macOS or another app`;
        settings.shortcut = a.shortcut as string;
        return settings.shortcut;
      case "suspend_shortcut":
        console.info(`kopi: shortcut ${a.suspended ? "suspended" : "resumed"}`);
        return null;
      case "set_capture_kinds":
        settings.captureKinds = a.kinds as ClipKind[];
        return null;
      case "set_theme":
        settings.theme = a.theme as Theme;
        void emit("theme:changed", { theme: settings.theme });
        return null;
      case "set_auto_update":
        settings.autoUpdate = a.enabled as boolean;
        return null;
      case "launch_at_login":
        return launchAtLogin;
      case "set_launch_at_login":
        launchAtLogin = a.enabled as boolean;
        return null;
      case "history_limits":
        return limits;
      case "set_history_limits":
        limits = a.limits as HistoryLimits;
        return null;
      case "clear_history":
        return clearHistory(a.range as ClearRange, a.keepPinned as boolean);
      case "excluded_apps":
        return [...excluded].sort((x, y) => x.name.localeCompare(y.name));
      case "add_excluded_app":
        if (!excluded.some((e) => e.id.toLowerCase() === (a.id as string).toLowerCase())) {
          excluded.push({ id: a.id as string, name: a.name as string, isDefault: false });
        }
        return null;
      case "remove_excluded_app":
        excluded = excluded.filter((e) => e.id.toLowerCase() !== (a.id as string).toLowerCase());
        return null;
      case "running_apps":
        return APPS.slice(0, 5).map(([id, name]) => ({ id, name }));
      case "permission_status":
        return {
          accessibility: granted !== null,
          pasteFromOtherApps: granted === null || granted === "some" ? "notAsked" : "allowed",
        };
      case "request_permission":
      case "open_permission_settings":
        console.info(`kopi: ${cmd} ${a.permission}`);
        return null;
      case "open_settings":
        window.open("/settings.html", "_blank");
        return null;
      case "finish_onboarding":
        console.info(`kopi: finish_onboarding${a.openPanel ? ", then open the panel" : ""}`);
        if (a.openPanel) location.assign("/");
        return null;
      case "sync_status":
        return structuredClone(sync);
      case "set_sync_enabled":
        sync.enabled = a.enabled as boolean;
        syncChanged();
        return null;
      case "set_sync_device_name":
        sync.deviceName = (a.name as string).trim() || sync.deviceName;
        syncChanged();
        return sync.deviceName;
      case "set_sync_snippets":
        sync.snippets = a.on as boolean;
        return null;
      case "set_sync_kinds":
        sync.kinds = a.kinds as ClipKind[];
        return null;
      case "remove_sync_device":
        sync.devices = sync.devices.filter((d) => d.id !== a.id);
        syncChanged();
        return null;
      case "start_pairing":
        sync.pairing = { code: "731406", expiresAt: Date.now() + 300_000, nearby: [] };
        syncChanged();
        // Another computer opens its sheet a moment later.
        setTimeout(() => {
          sync.pairing?.nearby.push({ id: "laptop", name: "Work laptop" });
          syncChanged();
        }, 1500);
        return null;
      case "stop_pairing":
        sync.pairing = null;
        syncChanged();
        return null;
      case "pair_with":
        if (a.code !== "482913") {
          throw "That code didn't match. Check the code on the other computer, which has a new one now.";
        }
        sync.devices.push({
          id: "laptop",
          name: "Work laptop",
          os: "windows",
          connected: true,
          needsUpdate: false,
          lastSeenAt: Date.now(),
          stale: false,
        });
        syncChanged();
        return "Work laptop";
      case "open_local_network_settings":
        console.info("kopi: open Local Network settings");
        return null;
      case "open_language_settings":
        console.info("kopi: open Language settings");
        return null;
      case "open_privacy_design":
        window.open("https://github.com/code-devanshu/kopi/blob/main/docs/PRIVACY_DESIGN.md", "_blank");
        return null;
    }
    throw `mock backend: no command ${cmd}`;
  },
  { shouldMockEvents: true },
);

const page = params.get("page");
if (page) sessionStorage.setItem("kopi.settings.page", page);

const step = params.get("step");
if (step) sessionStorage.setItem("kopi.onboarding.step", step);

const preview = params.get("preview");
if (preview) localStorage.setItem("kopi.preview", preview);

const theme = params.get("theme");
if (theme) document.documentElement.dataset.theme = theme;

// A dark page behind the transparent panel, as on a desktop.
if (!/\/(settings|onboarding)\.html$/.test(location.pathname) && !params.has("shot")) {
  document.documentElement.style.background = "#7a6a5c";
  document.body.style.padding = "0";
}

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
  const tab = params.get("tab");
  if (tab) document.querySelector<HTMLButtonElement>(`.tab[title="${CSS.escape(tab)}"]`)?.click();
  const select = Number(params.get("select") ?? 0);
  for (let i = 0; i < select; i++) {
    input?.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown", bubbles: true }));
  }
}, 300);
