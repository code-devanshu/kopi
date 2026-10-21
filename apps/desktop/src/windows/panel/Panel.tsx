import { useCallback, useEffect, useRef, useState, type KeyboardEvent, type PointerEvent } from "react";
import { Menu, type MenuEntry } from "../../components/Menu";
import { ClipList } from "../../features/history/ClipList";
import { History, rowAt, rowCount, rowId, type View } from "../../features/history/history";
import { actionFor } from "../../features/history/keys";
import { Empty, LoadError, LoadingRows } from "../../features/history/States";
import { useHistory } from "../../features/history/useHistory";
import { Paster, type Notice } from "../../features/paste/paster";
import { nextColor, reorder, stepView } from "../../features/pinboards/boards";
import { ColorRow } from "../../features/pinboards/ColorRow";
import { Switcher, type Editing } from "../../features/pinboards/Switcher";
import { useDrag, type DragItem, type DropTarget } from "../../features/pinboards/useDrag";
import { Preview } from "../../features/preview/Preview";
import { SearchBar } from "../../features/search/SearchBar";
import { useThemeSetting } from "../../features/settings/theme";
import { SnippetEditor, type EditorHandle } from "../../features/snippets/SnippetEditor";
import { SnippetForm } from "../../features/snippets/SnippetForm";
import { SnippetPreview } from "../../features/snippets/SnippetPreview";
import { fill, inputLabels, nameFrom, parse, usesClipboard, type Token } from "../../features/snippets/template";
import { KIND_LABELS } from "../../lib/format";
import {
  commands,
  events,
  unwrap,
  type ClipKind,
  type Pinboard,
  type SearchHit,
  type Snippet,
  type SnippetFields,
  type SourceApp,
} from "../../lib/ipc";
import { primary } from "../../lib/platform";
import { Hints } from "./Hints";
import { NoticeBar } from "./NoticeBar";

const LIST_ID = "clip-list";
/** Whether the preview is open, remembered between launches. */
const PREVIEW_KEY = "kopi.preview";
/** How often the "5m ago" times are brought up to date while the panel is open. */
const CLOCK_MS = 30_000;
/** How long a confirmation such as "Pinned" shows. */
const FLASH_MS = 1400;
/** The kinds of clip that can become a snippet: those that are text. */
const SNIPPET_KINDS: ClipKind[] = ["text", "rich_text", "link", "code"];

type OpenMenu = { label: string; x: number; y: number; entries: MenuEntry[]; initialActive?: number };

/** The snippet editor, in place of the list. `key` makes each opening a fresh editor. */
type OpenEditor = { key: number; snippet: Snippet | null; initial: SnippetFields; others: Snippet[] };

/** What rows, the drag and the menus call. */
type Handlers = {
  pick: (index: number, plain: boolean) => void;
  drop: (item: DragItem, target: DropTarget) => void;
  drag: (index: number, e: PointerEvent) => void;
  menu: (index: number, x: number, y: number) => void;
};

/** The `{input}` form, before a snippet pastes or copies. */
type Filling = { snippet: Snippet; tokens: Token[]; labels: string[]; action: "paste" | "copy" };

export function Panel() {
  useThemeSetting();
  const [history] = useState(
    () =>
      new History((query) => unwrap(commands.searchClips(query)), {
        all: (text) => unwrap(commands.snippetSearch(text)),
        keyword: (text) => unwrap(commands.snippetKeyword(text)),
      }),
  );
  const state = useHistory(history);
  const inputRef = useRef<HTMLInputElement>(null);
  const editorRef = useRef<EditorHandle>(null);
  const pageSize = useRef(8);
  const editorCount = useRef(0);
  const [apps, setApps] = useState<SourceApp[]>([]);
  const [boards, setBoards] = useState<Pinboard[]>([]);
  const [editing, setEditing] = useState<Editing | null>(null);
  const [editor, setEditor] = useState<OpenEditor | null>(null);
  const [filling, setFilling] = useState<Filling | null>(null);
  const [menu, setMenu] = useState<OpenMenu | null>(null);
  const [now, setNow] = useState(() => Date.now());
  const [previewOpen, setPreviewOpen] = useState(() => load(PREVIEW_KEY) !== "closed");
  // Bumped when a clip or snippet changes, so the preview fetches it again.
  const [version, setVersion] = useState(0);
  // Per content hash, how many times its thumbnail was written. Rows load it again on a bump.
  const [thumbnails, setThumbnails] = useState<ReadonlyMap<string, number>>(new Map());
  const [notice, setNotice] = useState<Notice | null>(null);
  /** Sync is on with another computer, so deleting a snippet deletes it there too. */
  const [syncing, setSyncing] = useState(false);
  // A plain holder, not a ref: the Paster made during the first render closes over it.
  const [flashTimer] = useState<{ current?: ReturnType<typeof setTimeout> }>(() => ({}));
  const [paster] = useState(
    () =>
      new Paster({
        paste: (id, plain) => unwrap(commands.pasteClip(id, plain)),
        copy: (id, plain) => unwrap(commands.copyClip(id, plain)),
        hide: () => void commands.hidePanel(),
        openAccessibility: () => void commands.openPermissionSettings("accessibility"),
        notify: (n) => {
          clearTimeout(flashTimer.current);
          setNotice(n);
        },
      }),
  );
  // Whether the editor is open, for the panel:shown listener, which is set up once.
  const editorOpen = useRef(false);
  useEffect(() => {
    editorOpen.current = editor !== null;
  }, [editor]);

  /** A message that goes away by itself. */
  const flash = useCallback(
    (n: Notice) => {
      clearTimeout(flashTimer.current);
      setNotice(n);
      flashTimer.current = setTimeout(() => setNotice(null), n.tone === "error" ? FLASH_MS * 3 : FLASH_MS);
    },
    [flashTimer],
  );
  const failed = useCallback(
    (what: string) => (e: unknown) => flash({ message: `Couldn't ${what}.`, detail: String(e), tone: "error" }),
    [flash],
  );

  const loadBoards = useCallback(
    () =>
      unwrap(commands.pinboardList()).then((list) => {
        setBoards(list);
        // The pinboard on show was deleted.
        const view = history.getSnapshot().view;
        if (typeof view === "number" && !list.some((b) => b.id === view)) history.setView("history");
      }, failed("load your pinboards")),
    [history, failed],
  );

  // Rows, the drag and the menus keep one callback each, which reaches the current functions
  // through these. They're set after every render, once the functions are declared.
  const latest = useRef<Handlers>({ pick: () => {}, drop: () => {}, drag: () => {}, menu: () => {} });
  const { drag, start: startDrag, cancel: cancelDrag } = useDrag((item, target) => latest.current.drop(item, target));

  useEffect(() => {
    const loadApps = () => unwrap(commands.sourceApps()).then(setApps, () => {});
    const loadSync = () =>
      unwrap(commands.syncStatus()).then(
        (s) => setSyncing(s.enabled && s.devices.length > 0),
        () => {},
      );
    void history.reset();
    loadApps();
    void loadBoards();
    void loadSync();

    const shown = () => {
      paster.reset();
      setNow(Date.now());
      void history.reset();
      loadApps();
      void loadBoards();
      // The webview has focus a frame after the panel is shown. The editor keeps its own.
      requestAnimationFrame(() => {
        if (!editorOpen.current) inputRef.current?.focus();
      });
    };
    const hidden = () => {
      paster.reset();
      setMenu(null);
      // The app in front may have changed, so the blanks are asked for again next time.
      setFilling(null);
      cancelDrag();
    };
    const changed = () => {
      history.refreshSoon();
      setVersion((v) => v + 1);
    };
    const listeners = [
      events.panelShown.listen(shown),
      events.panelHidden.listen(hidden),
      events.clipAdded.listen(() => {
        history.refreshSoon();
        // A snippet's {clipboard} may now be this.
        setVersion((v) => v + 1);
      }),
      events.clipUpdated.listen(changed),
      events.clipRemoved.listen(() => {
        changed();
        // Its pinboards hold one fewer.
        void loadBoards();
      }),
      events.pinboardsChanged.listen(() => {
        history.refreshSoon();
        void loadBoards();
      }),
      events.snippetsChanged.listen(changed),
      events.syncChanged.listen(() => void loadSync()),
      events.thumbnailReady.listen(({ payload: { contentHash } }) =>
        setThumbnails((m) => new Map(m).set(contentHash, (m.get(contentHash) ?? 0) + 1)),
      ),
    ];
    const clock = setInterval(() => setNow(Date.now()), CLOCK_MS);
    return () => {
      clearInterval(clock);
      clearTimeout(flashTimer.current);
      listeners.forEach((unlisten) => void unlisten.then((stop) => stop()));
      paster.reset();
    };
  }, [history, paster, loadBoards, cancelDrag, flashTimer]);

  useEffect(() => save(PREVIEW_KEY, previewOpen ? "open" : "closed"), [previewOpen]);

  const refocus = () => inputRef.current?.focus();
  const boardName = (id: number) => boards.find((b) => b.id === id)?.name ?? "the pinboard";

  const select = useCallback((index: number) => history.select(index), [history]);
  const pick = useCallback((index: number, plain = false) => latest.current.pick(index, plain), []);
  const loadMore = useCallback(() => void history.loadMore(), [history]);
  const setPageSize = useCallback((rows: number) => {
    pageSize.current = rows;
  }, []);

  function pickRow(index: number, plain: boolean) {
    const row = rowAt(history.getSnapshot(), index);
    if (!row) return;
    history.select(index);
    if (row.snippet) return sendSnippet(row.snippet, "paste");
    void paster.paste(row.hit.id, plain);
  }

  // Clips. The list and the pinboard counts update from the events each command sends.

  function setPinned(hit: SearchHit, pinned: boolean) {
    unwrap(commands.setPinned(hit.id, pinned)).then(
      () => flash({ message: pinned ? "Pinned." : "Unpinned.", tone: "info" }),
      failed(pinned ? "pin it" : "unpin it"),
    );
  }

  function addToBoard(board: number, clip: number) {
    unwrap(commands.pinboardAddClips(board, [clip])).then(
      () => flash({ message: `Added to ${boardName(board)}.`, tone: "info" }),
      failed("add it to the pinboard"),
    );
  }

  function removeFromBoard(board: number, clip: number) {
    unwrap(commands.pinboardRemoveClips(board, [clip])).then(
      () => flash({ message: `Removed from ${boardName(board)}.`, tone: "info" }),
      failed("take it off the pinboard"),
    );
  }

  function deleteClip(hit: SearchHit) {
    // The list updates from the clip:removed event.
    unwrap(commands.deleteClips([hit.id])).catch(() => history.refresh());
  }

  // Snippets.

  /** Pastes or copies a snippet, asking for its blanks first if it has any. */
  function sendSnippet(snippet: Snippet, action: "paste" | "copy") {
    const tokens = parse(snippet.body);
    const labels = inputLabels(tokens);
    if (labels.length) return setFilling({ snippet, tokens, labels, action });
    void deliver(snippet, tokens, {}, action);
  }

  async function deliver(snippet: Snippet, tokens: Token[], inputs: Record<string, string>, action: "paste" | "copy") {
    const filled = async () =>
      fill(tokens, {
        now: new Date(),
        clipboard: usesClipboard(tokens) ? await unwrap(commands.clipboardText()) : null,
        inputs,
      });
    if (action === "paste") {
      await paster.pasteWith(async () => {
        const { text, cursorBack } = await filled();
        return unwrap(commands.pasteSnippet(snippet.id, text, cursorBack));
      });
    } else {
      await paster.copyWith(async () => unwrap(commands.copySnippet(snippet.id, (await filled()).text)));
    }
  }

  function submitForm(inputs: Record<string, string>) {
    const was = filling;
    setFilling(null);
    refocus();
    if (was) void deliver(was.snippet, was.tokens, inputs, was.action);
  }

  async function openEditor(snippet: Snippet | null, initial?: SnippetFields) {
    setMenu(null);
    if (editor) await closeEditor(null);
    const others = await unwrap(commands.snippetSearch("")).catch(() => [] as Snippet[]);
    setEditor({
      key: ++editorCount.current,
      snippet,
      initial: initial ?? (snippet ? fieldsOf(snippet) : { name: "", keyword: null, body: "" }),
      others,
    });
  }

  /**
   * Saves and closes the editor. Then `goTo` is shown, or with none, the snippet is selected,
   * in the Snippets tab if it's new.
   */
  async function closeEditor(goTo?: View | null) {
    const was = editor;
    const id = (await editorRef.current?.flush()) ?? null;
    setEditor(null);
    refocus();
    if (goTo !== undefined && goTo !== null) return history.setView(goTo);
    if (goTo === null || id === null) return;
    if (!was?.snippet) history.setView("snippets");
    await history.refresh();
    selectSnippet(id);
  }

  function selectSnippet(id: number) {
    const at = history.getSnapshot().snippets.findIndex((s) => s.id === id);
    if (at >= 0) history.select(at);
  }

  async function saveAsSnippet(hit: SearchHit) {
    try {
      const body = (await unwrap(commands.getClip(hit.id)))?.text ?? "";
      if (!body) return;
      history.setView("snippets");
      await openEditor(null, { name: nameFrom(body), keyword: null, body });
    } catch (e) {
      failed("make the snippet")(e);
    }
  }

  async function duplicateSnippet(snippet: Snippet) {
    try {
      const copy = await unwrap(
        commands.snippetCreate({ name: `${[...snippet.name].slice(0, 59).join("")} copy`, keyword: null, body: snippet.body }),
      );
      await history.refresh();
      selectSnippet(copy.id);
    } catch (e) {
      failed("duplicate the snippet")(e);
    }
  }

  function deleteSnippet(snippet: Snippet) {
    unwrap(commands.snippetDelete(snippet.id)).then(
      () => flash({ message: `Deleted ${snippet.name}.`, tone: "info" }),
      failed("delete the snippet"),
    );
  }

  /** Moves a snippet to where `target` is, in the full list. */
  function moveSnippet(id: number, target: number | undefined) {
    if (target === undefined) return;
    const ids = history.getSnapshot().snippets.map((s) => s.id);
    const next = reorder(ids, id, target);
    if (next === ids) return;
    unwrap(commands.snippetReorder(next)).catch(failed("move the snippet"));
  }

  /** Reordering only makes sense with every snippet listed. */
  function inFullList() {
    const { view, text } = history.getSnapshot();
    return view === "snippets" && !text.trim();
  }

  // Pinboards.

  async function commitName(name: string) {
    const was = editing;
    setEditing(null);
    refocus();
    if (!was) return;
    try {
      if (was.type === "rename") {
        await unwrap(commands.pinboardRename(was.id, name));
        return;
      }
      const board = await unwrap(commands.pinboardCreate(name, nextColor(boards)));
      if (was.addClip !== undefined) {
        await unwrap(commands.pinboardAddClips(board.id, [was.addClip]));
        flash({ message: `Added to ${board.name}.`, tone: "info" });
      } else {
        await loadBoards();
        history.setView(board.id);
      }
    } catch (e) {
      failed(was.type === "new" ? "make the pinboard" : "rename the pinboard")(e);
    }
  }

  function cancelName() {
    setEditing(null);
    refocus();
  }

  function moveBoard(id: number, target: number) {
    const ids = boards.map((b) => b.id);
    const next = reorder(ids, id, target);
    if (next === ids) return;
    // Show the new order straight away. pinboards:changed brings the stored one.
    setBoards(next.map((i) => boards.find((b) => b.id === i)!));
    unwrap(commands.pinboardReorder(next)).catch((e) => {
      failed("move the pinboard")(e);
      void loadBoards();
    });
  }

  function drop(item: DragItem, target: DropTarget) {
    if (!canDrop(item, target)) return;
    if (item.type === "board") return moveBoard(item.id, target as number);
    if (item.type === "snippet") return moveSnippet(item.id, (target as { snippet: number }).snippet);
    const hit = history.getSnapshot().hits.find((h) => h.id === item.id);
    if (target === "pinned") {
      if (hit && !hit.pinned) setPinned(hit, true);
      return;
    }
    if (target === "snippets") {
      if (hit) void saveAsSnippet(hit);
      return;
    }
    addToBoard(target as number, item.id);
  }

  // Menus.

  function closeMenu() {
    setMenu(null);
    refocus();
  }

  async function openRowMenu(index: number, x: number, y: number) {
    const row = rowAt(history.getSnapshot(), index);
    if (!row) return;
    history.select(index);
    if (row.snippet) return openSnippetMenu(row.snippet, x, y);
    const hit = row.hit;
    const on = await unwrap(commands.clipPinboards(hit.id)).catch(() => [] as number[]);
    const view = history.getSnapshot().view;
    const inBoard = typeof view === "number" ? view : null;
    const entries: MenuEntry[] = [
      { type: "item", label: "Paste", shortcut: "↩", run: () => void paster.paste(hit.id, false) },
      { type: "item", label: "Paste as plain text", shortcut: "⌥↩", run: () => void paster.paste(hit.id, true) },
      { type: "item", label: "Copy", shortcut: "⌘C", run: () => void paster.copy(hit.id, false) },
      { type: "separator" },
      {
        type: "item",
        label: hit.pinned ? "Unpin" : "Pin",
        shortcut: "⌘P",
        run: () => setPinned(hit, !hit.pinned),
      },
      ...(SNIPPET_KINDS.includes(hit.kind)
        ? [{ type: "item", label: "Save as snippet…", run: () => void saveAsSnippet(hit) } satisfies MenuEntry]
        : []),
      { type: "heading", label: "Pinboards" },
      ...boards.map(
        (b): MenuEntry => ({
          type: "item",
          label: b.name,
          color: b.color,
          checked: on.includes(b.id),
          run: () => (on.includes(b.id) ? removeFromBoard(b.id, hit.id) : addToBoard(b.id, hit.id)),
        }),
      ),
      { type: "item", label: "New pinboard…", run: () => setEditing({ type: "new", addClip: hit.id }) },
      { type: "separator" },
      ...(inBoard !== null
        ? [
            {
              type: "item",
              label: `Remove from ${boardName(inBoard)}`,
              shortcut: "⌘⌫",
              run: () => removeFromBoard(inBoard, hit.id),
            } satisfies MenuEntry,
          ]
        : []),
      {
        type: "item",
        label: "Delete",
        shortcut: inBoard === null ? "⌘⌫" : undefined,
        danger: true,
        run: () => deleteClip(hit),
      },
    ];
    setMenu({ label: "Clip actions", x, y, entries });
  }

  function openSnippetMenu(snippet: Snippet, x: number, y: number) {
    const list = history.getSnapshot().snippets;
    const at = list.findIndex((s) => s.id === snippet.id);
    const movable = inFullList();
    setMenu({
      label: `${snippet.name} actions`,
      x,
      y,
      entries: [
        { type: "item", label: "Paste", shortcut: "↩", run: () => sendSnippet(snippet, "paste") },
        { type: "item", label: "Copy", shortcut: "⌘C", run: () => sendSnippet(snippet, "copy") },
        { type: "separator" },
        { type: "item", label: "Edit", shortcut: "⌘E", run: () => void openEditor(snippet) },
        { type: "item", label: "Duplicate", run: () => void duplicateSnippet(snippet) },
        { type: "separator" },
        {
          type: "item",
          label: "Move up",
          disabled: !movable || at <= 0,
          run: () => moveSnippet(snippet.id, list[at - 1]?.id),
        },
        {
          type: "item",
          label: "Move down",
          disabled: !movable || at >= list.length - 1,
          run: () => moveSnippet(snippet.id, list[at + 1]?.id),
        },
        { type: "separator" },
        {
          type: "item",
          label: "Delete",
          shortcut: "⌘⌫",
          confirm: "Delete? It can’t be undone",
          danger: true,
          run: () => deleteSnippet(snippet),
        },
      ],
    });
  }

  /** ⌘⌫ on a snippet asks first. */
  function confirmDeleteSnippet(snippet: Snippet) {
    const { x, y } = rowPoint(`snippet-${snippet.id}`);
    const entries: MenuEntry[] = [{ type: "heading", label: `Delete “${snippet.name}”?` }];
    if (syncing) entries.push({ type: "heading", label: "This removes it from your other computers too." });
    // Delete is highlighted, so Return deletes.
    const initialActive = entries.length;
    entries.push(
      { type: "item", label: "Delete", shortcut: "↩", danger: true, run: () => deleteSnippet(snippet) },
      { type: "item", label: "Cancel", shortcut: "esc", run: () => {} },
    );
    setMenu({ label: `Delete ${snippet.name}`, x, y, initialActive, entries });
  }

  /** ⌘K: the selected row's menu, under it. */
  function openSelectedMenu() {
    const row = rowAt(state, state.selected);
    if (!row) return;
    const { x, y } = rowPoint(rowId(row));
    void openRowMenu(state.selected, x, y);
  }

  function openBoardMenu(board: Pinboard, x: number, y: number) {
    const at = boards.findIndex((b) => b.id === board.id);
    const neighbour = (by: number) => boards[at + by]?.id;
    setMenu({
      label: `${board.name} actions`,
      x,
      y,
      entries: [
        { type: "item", label: "Rename", run: () => setEditing({ type: "rename", id: board.id }) },
        {
          type: "custom",
          key: "colors",
          node: (
            <ColorRow
              value={board.color}
              onPick={(color) => {
                closeMenu();
                unwrap(commands.pinboardRecolor(board.id, color)).catch(failed("change the color"));
              }}
            />
          ),
        },
        { type: "separator" },
        {
          type: "item",
          label: "Move left",
          disabled: at <= 0,
          run: () => moveBoard(board.id, neighbour(-1)),
        },
        {
          type: "item",
          label: "Move right",
          disabled: at >= boards.length - 1,
          run: () => moveBoard(board.id, neighbour(1)),
        },
        { type: "separator" },
        {
          type: "item",
          label: "Delete pinboard",
          confirm: "Delete? Its clips stay in History",
          danger: true,
          run: () => void unwrap(commands.pinboardDelete(board.id)).catch(failed("delete the pinboard")),
        },
      ],
    });
  }

  // Dragging.

  function canDrop(item: DragItem, target: DropTarget) {
    if (item.type === "board") return typeof target === "number" && target !== item.id;
    if (item.type === "snippet") return typeof target === "object" && target.snippet !== item.id;
    if (target === "snippets") {
      const hit = history.getSnapshot().hits.find((h) => h.id === item.id);
      return hit !== undefined && SNIPPET_KINDS.includes(hit.kind);
    }
    return target === "pinned" || typeof target === "number";
  }

  function dragFromRow(index: number, e: PointerEvent) {
    const row = rowAt(history.getSnapshot(), index);
    if (row?.hit) startDrag(e, { type: "clip", id: row.hit.id, label: dragLabel(row.hit) });
    else if (row?.snippet && inFullList()) startDrag(e, { type: "snippet", id: row.snippet.id, label: row.snippet.name });
  }

  useEffect(() => {
    latest.current = {
      pick: pickRow,
      drop,
      drag: dragFromRow,
      // With the current pinboards.
      menu: (index, x, y) => void openRowMenu(index, x, y),
    };
  });
  const dragRow = useCallback((index: number, e: PointerEvent) => latest.current.drag(index, e), []);
  const openMenuAt = useCallback((index: number, x: number, y: number) => latest.current.menu(index, x, y), []);

  function onKeyDown(e: KeyboardEvent) {
    // After "Copied. Press ⌘V", ⌘V would paste into the search box, so it closes the panel
    // and the next ⌘V reaches the app.
    if (paster.closing && primary(e) && e.code === "KeyV") {
      e.preventDefault();
      return paster.closeNow();
    }
    const action = actionFor(e.nativeEvent, state.text !== "");
    if (!action) return;
    // A focused filter keeps its own arrow keys.
    if (e.target instanceof HTMLSelectElement && action.type !== "hide") return;
    // ⌘C with text selected in the search box copies that text.
    if (action.type === "copy" && hasSelection(e.target)) return;
    e.preventDefault();
    const row = rowAt(state, state.selected);
    const hit = row?.hit;
    const snippet = row?.snippet;
    switch (action.type) {
      case "move":
        return history.move(action.by);
      case "page":
        return history.move(action.by * pageSize.current);
      case "first":
        return history.select(0);
      case "last":
        return history.select(rowCount(state) - 1);
      case "paste":
        return pick(state.selected, action.plain);
      case "quickPaste":
        return pick(action.index, action.plain);
      case "copy":
        if (snippet) return sendSnippet(snippet, "copy");
        if (hit) void paster.copy(hit.id, action.plain);
        return;
      case "delete":
        if (snippet) return confirmDeleteSnippet(snippet);
        if (!hit) return;
        // In a pinboard, ⌘⌫ takes the clip off it. It stays in the history.
        if (typeof state.view === "number") return removeFromBoard(state.view, hit.id);
        return deleteClip(hit);
      case "pin":
        if (hit) setPinned(hit, !hit.pinned);
        return;
      case "menu":
        return openSelectedMenu();
      case "newSnippet":
        return void openEditor(null);
      case "edit":
        if (snippet) void openEditor(snippet);
        return;
      case "view":
        return history.setView(stepView(boards, state.view, action.by));
      case "togglePreview":
        return setPreviewOpen((open) => !open);
      case "settings":
        return void unwrap(commands.openSettings()).catch(failed("open Settings"));
      case "hide":
        return void commands.hidePanel();
    }
  }

  const inSnippets = state.view === "snippets";
  const selectedRow = rowAt(state, state.selected);
  // With nothing to show, the list and its message get the whole width.
  const showPreview = previewOpen && selectedRow !== undefined;
  const appName = state.appId
    ? (apps.find((a) => a.id === state.appId)?.name ?? state.appId)
    : null;
  const dropSnippet =
    drag?.item.type === "snippet" && typeof drag.over === "object" && drag.over !== null && canDrop(drag.item, drag.over)
      ? drag.over.snippet
      : null;

  let results;
  if (!state.loaded) {
    results = <LoadingRows />;
  } else if (state.error && !rowCount(state)) {
    results = <LoadError message={state.error} onRetry={() => void history.refresh()} />;
  } else if (!rowCount(state)) {
    results = (
      <Empty
        place={
          state.view === "pinned" || state.view === "snippets"
            ? state.view
            : typeof state.view === "number"
              ? { board: boardName(state.view) }
              : null
        }
        text={state.text}
        kind={state.kind}
        appName={appName}
        onClearFilters={() => {
          history.setKind(null);
          history.setApp(null);
        }}
        onNewSnippet={() => void openEditor(null)}
      />
    );
  } else {
    results = (
      <ClipList
        id={LIST_ID}
        searchKey={`${state.view}\n${state.text}\n${state.kind}\n${state.appId}`}
        snippets={state.snippets}
        hits={state.hits}
        query={state.text}
        labelSnippets={!inSnippets}
        dropSnippet={dropSnippet}
        selected={state.selected}
        now={now}
        thumbnailAttempts={thumbnails}
        onSelect={select}
        onPick={pick}
        onMenu={openMenuAt}
        onDragStart={dragRow}
        onNearEnd={loadMore}
        onPageSize={setPageSize}
      />
    );
  }

  let content;
  if (editor) {
    content = (
      <div className="content">
        <SnippetEditor
          key={editor.key}
          ref={editorRef}
          snippet={editor.snippet}
          initial={editor.initial}
          others={editor.others}
          onDone={() => void closeEditor()}
        />
      </div>
    );
  } else if (filling) {
    content = (
      <div className="content">
        <SnippetForm
          snippet={filling.snippet}
          labels={filling.labels}
          action={filling.action}
          onSubmit={submitForm}
          onCancel={() => {
            setFilling(null);
            refocus();
          }}
        />
      </div>
    );
  } else {
    content = (
      <div className={showPreview ? "content with-preview" : "content"}>
        <div className="results">{results}</div>
        {showPreview &&
          (selectedRow.snippet ? (
            <SnippetPreview snippet={selectedRow.snippet} now={now} version={version} />
          ) : (
            <Preview id={selectedRow.hit.id} version={version} query={state.text} />
          ))}
      </div>
    );
  }

  return (
    <div className="panel" onKeyDown={onKeyDown}>
      <SearchBar
        inputRef={inputRef}
        text={state.text}
        kind={state.kind}
        appId={state.appId}
        apps={apps}
        listId={LIST_ID}
        activeId={selectedRow ? rowId(selectedRow) : undefined}
        filters={!inSnippets}
        placeholder={inSnippets ? "Search snippets" : "Search clips"}
        onText={(text) => {
          if (editor) void closeEditor(null);
          setFilling(null);
          history.setText(text);
        }}
        onKind={(kind) => history.setKind(kind)}
        onApp={(appId) => history.setApp(appId)}
      />
      <Switcher
        boards={boards}
        view={state.view}
        editing={editing}
        drag={drag}
        canDrop={(target) => drag !== null && canDrop(drag.item, target)}
        onView={(view) => {
          setFilling(null);
          if (editor) return void closeEditor(view);
          history.setView(view);
        }}
        onNew={() => setEditing({ type: "new" })}
        onRename={(id) => setEditing({ type: "rename", id })}
        onCommitName={(name) => void commitName(name)}
        onCancelName={cancelName}
        onBoardMenu={openBoardMenu}
        onBoardPointerDown={(e, board) => startDrag(e, { type: "board", id: board.id, label: board.name })}
      />
      {content}
      <NoticeBar notice={notice} />
      <Hints
        typing={state.text !== ""}
        previewOpen={previewOpen}
        inBoard={typeof state.view === "number"}
        inSnippets={inSnippets}
      />
      {menu && <Menu {...menu} onClose={closeMenu} />}
      {drag && (
        <div className="drag-ghost" style={{ left: drag.x, top: drag.y }} aria-hidden="true">
          {drag.item.label}
        </div>
      )}
    </div>
  );
}

function fieldsOf(snippet: Snippet): SnippetFields {
  return { name: snippet.name, keyword: snippet.keyword, body: snippet.body };
}

/** Where a menu for the row with element id `id` opens: just under it. */
function rowPoint(id: string): { x: number; y: number } {
  const row = document.getElementById(id)?.getBoundingClientRect();
  return { x: row ? row.left + 52 : 60, y: row ? row.bottom - 6 : 120 };
}

/** A short name for a clip being dragged. */
function dragLabel(hit: SearchHit): string {
  const text = hit.snippet
    .map((s) => s.text)
    .join("")
    .replace(/\s+/g, " ")
    .trim();
  if (!text) return KIND_LABELS[hit.kind];
  return text.length > 40 ? `${text.slice(0, 40)}…` : text;
}

function hasSelection(target: EventTarget) {
  return target instanceof HTMLInputElement && target.selectionStart !== target.selectionEnd;
}

// The preview setting is a convenience: without storage, the preview starts open.
function load(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}

function save(key: string, value: string) {
  try {
    localStorage.setItem(key, value);
  } catch {
    // Not saved. It's only a convenience.
  }
}
