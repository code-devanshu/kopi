import { useCallback, useEffect, useRef, useState, type KeyboardEvent, type PointerEvent } from "react";
import { Menu, type MenuEntry } from "../../components/Menu";
import { ClipList } from "../../features/history/ClipList";
import { History, type View } from "../../features/history/history";
import { actionFor } from "../../features/history/keys";
import { Empty, LoadError, LoadingRows } from "../../features/history/States";
import { useHistory } from "../../features/history/useHistory";
import { Paster, type Notice } from "../../features/paste/paster";
import { nextColor, reorder, stepView } from "../../features/pinboards/boards";
import { ColorRow } from "../../features/pinboards/ColorRow";
import { Switcher, type Editing } from "../../features/pinboards/Switcher";
import { useDrag, type DragItem } from "../../features/pinboards/useDrag";
import { Preview } from "../../features/preview/Preview";
import { SearchBar } from "../../features/search/SearchBar";
import { KIND_LABELS } from "../../lib/format";
import { commands, events, unwrap, type Pinboard, type SearchHit, type SourceApp } from "../../lib/ipc";
import { Hints } from "./Hints";
import { NoticeBar } from "./NoticeBar";

const LIST_ID = "clip-list";
/** Whether the preview is open, remembered between launches. */
const PREVIEW_KEY = "kopi.preview";
/** How often the "5m ago" times are brought up to date while the panel is open. */
const CLOCK_MS = 30_000;
/** How long a confirmation such as "Pinned" shows. */
const FLASH_MS = 1400;

type OpenMenu = { label: string; x: number; y: number; entries: MenuEntry[] };

export function Panel() {
  const [history] = useState(() => new History((query) => unwrap(commands.searchClips(query))));
  const state = useHistory(history);
  const inputRef = useRef<HTMLInputElement>(null);
  const pageSize = useRef(8);
  const [apps, setApps] = useState<SourceApp[]>([]);
  const [boards, setBoards] = useState<Pinboard[]>([]);
  const [editing, setEditing] = useState<Editing | null>(null);
  const [menu, setMenu] = useState<OpenMenu | null>(null);
  const [now, setNow] = useState(() => Date.now());
  const [previewOpen, setPreviewOpen] = useState(() => load(PREVIEW_KEY) !== "closed");
  // Bumped when a clip changes, so the preview fetches it again.
  const [version, setVersion] = useState(0);
  // Per content hash, how many times its thumbnail was written. Rows load it again on a bump.
  const [thumbnails, setThumbnails] = useState<ReadonlyMap<string, number>>(new Map());
  const [notice, setNotice] = useState<Notice | null>(null);
  const flashTimer = useRef<ReturnType<typeof setTimeout>>(undefined);
  const [paster] = useState(
    () =>
      new Paster({
        paste: (id, plain) => unwrap(commands.pasteClip(id, plain)),
        copy: (id, plain) => unwrap(commands.copyClip(id, plain)),
        hide: () => void commands.hidePanel(),
        notify: (n) => {
          clearTimeout(flashTimer.current);
          setNotice(n);
        },
      }),
  );

  /** A message that goes away by itself. */
  const flash = useCallback((n: Notice) => {
    clearTimeout(flashTimer.current);
    setNotice(n);
    flashTimer.current = setTimeout(() => setNotice(null), n.tone === "error" ? FLASH_MS * 3 : FLASH_MS);
  }, []);
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

  const { drag, start: startDrag, cancel: cancelDrag } = useDrag((item, target) => drop(item, target));

  useEffect(() => {
    const loadApps = () => unwrap(commands.sourceApps()).then(setApps, () => {});
    void history.reset();
    loadApps();
    void loadBoards();

    const shown = () => {
      paster.reset();
      setNow(Date.now());
      void history.reset();
      loadApps();
      void loadBoards();
      // The webview has focus a frame after the panel is shown.
      requestAnimationFrame(() => inputRef.current?.focus());
    };
    const hidden = () => {
      paster.reset();
      setMenu(null);
      cancelDrag();
    };
    const changed = () => {
      history.refreshSoon();
      setVersion((v) => v + 1);
    };
    const listeners = [
      events.panelShown.listen(shown),
      events.panelHidden.listen(hidden),
      events.clipAdded.listen(() => history.refreshSoon()),
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
  }, [history, paster, loadBoards, cancelDrag]);

  useEffect(() => save(PREVIEW_KEY, previewOpen ? "open" : "closed"), [previewOpen]);

  const refocus = () => inputRef.current?.focus();
  const boardName = (id: number) => boards.find((b) => b.id === id)?.name ?? "the pinboard";

  const select = useCallback((index: number) => history.select(index), [history]);
  const pick = useCallback(
    (index: number, plain = false) => {
      const hit = history.getSnapshot().hits[index];
      if (!hit) return;
      history.select(index);
      void paster.paste(hit.id, plain);
    },
    [history, paster],
  );
  const loadMore = useCallback(() => void history.loadMore(), [history]);
  const setPageSize = useCallback((rows: number) => {
    pageSize.current = rows;
  }, []);

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

  function drop(item: DragItem, target: View) {
    if (!canDrop(item, target)) return;
    if (item.type === "board") return moveBoard(item.id, target as number);
    if (target === "pinned") {
      const hit = history.getSnapshot().hits.find((h) => h.id === item.id);
      if (hit && !hit.pinned) setPinned(hit, true);
      return;
    }
    addToBoard(target as number, item.id);
  }

  // Menus.

  function closeMenu() {
    setMenu(null);
    refocus();
  }

  async function openClipMenu(index: number, x: number, y: number) {
    const hit = history.getSnapshot().hits[index];
    if (!hit) return;
    history.select(index);
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

  /** ⌘K: the selected clip's menu, under its row. */
  function openSelectedMenu() {
    const hit = history.selectedHit;
    if (!hit) return;
    const row = document.getElementById(`clip-${hit.id}`)?.getBoundingClientRect();
    void openClipMenu(state.selected, row ? row.left + 52 : 60, row ? row.bottom - 6 : 120);
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

  function canDrop(item: DragItem, target: View) {
    if (item.type === "board") return typeof target === "number" && target !== item.id;
    return target === "pinned" || typeof target === "number";
  }

  const dragClip = useCallback(
    (index: number, e: PointerEvent) => {
      const hit = history.getSnapshot().hits[index];
      if (hit) startDrag(e, { type: "clip", id: hit.id, label: dragLabel(hit) });
    },
    [history, startDrag],
  );
  // Rows keep one callback, which opens the menu with the current pinboards.
  const clipMenu = useRef(openClipMenu);
  useEffect(() => {
    clipMenu.current = openClipMenu;
  });
  const openMenuAt = useCallback(
    (index: number, x: number, y: number) => void clipMenu.current(index, x, y),
    [],
  );

  function onKeyDown(e: KeyboardEvent) {
    // After "Copied. Press ⌘V", ⌘V would paste into the search box, so it closes the panel
    // and the next ⌘V reaches the app.
    if (paster.closing && e.metaKey && e.code === "KeyV") {
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
    const hit = history.selectedHit;
    switch (action.type) {
      case "move":
        return history.move(action.by);
      case "page":
        return history.move(action.by * pageSize.current);
      case "first":
        return history.select(0);
      case "last":
        return history.select(state.hits.length - 1);
      case "paste":
        return pick(state.selected, action.plain);
      case "quickPaste":
        return pick(action.index, action.plain);
      case "copy":
        if (hit) void paster.copy(hit.id, action.plain);
        return;
      case "delete":
        if (!hit) return;
        // In a pinboard, ⌘⌫ takes the clip off it. It stays in the history.
        if (typeof state.view === "number") return removeFromBoard(state.view, hit.id);
        return deleteClip(hit);
      case "pin":
        if (hit) setPinned(hit, !hit.pinned);
        return;
      case "menu":
        return openSelectedMenu();
      case "view":
        return history.setView(stepView(boards, state.view, action.by));
      case "togglePreview":
        return setPreviewOpen((open) => !open);
      case "hide":
        return void commands.hidePanel();
    }
  }

  const selectedHit = state.hits[state.selected];
  // With nothing to show, the list and its message get the whole width.
  const showPreview = previewOpen && selectedHit !== undefined;
  const appName = state.appId
    ? (apps.find((a) => a.id === state.appId)?.name ?? state.appId)
    : null;

  let results;
  if (!state.loaded) {
    results = <LoadingRows />;
  } else if (state.error && !state.hits.length) {
    results = <LoadError message={state.error} onRetry={() => void history.refresh()} />;
  } else if (!state.hits.length) {
    results = (
      <Empty
        place={
          state.view === "pinned"
            ? "pinned"
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
      />
    );
  } else {
    results = (
      <ClipList
        id={LIST_ID}
        searchKey={`${state.view}\n${state.text}\n${state.kind}\n${state.appId}`}
        hits={state.hits}
        selected={state.selected}
        now={now}
        thumbnailAttempts={thumbnails}
        onSelect={select}
        onPick={pick}
        onMenu={openMenuAt}
        onDragStart={dragClip}
        onNearEnd={loadMore}
        onPageSize={setPageSize}
      />
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
        activeId={selectedHit ? `clip-${selectedHit.id}` : undefined}
        onText={(text) => history.setText(text)}
        onKind={(kind) => history.setKind(kind)}
        onApp={(appId) => history.setApp(appId)}
      />
      <Switcher
        boards={boards}
        view={state.view}
        editing={editing}
        drag={drag}
        canDrop={(target) => drag !== null && canDrop(drag.item, target)}
        onView={(view) => history.setView(view)}
        onNew={() => setEditing({ type: "new" })}
        onRename={(id) => setEditing({ type: "rename", id })}
        onCommitName={(name) => void commitName(name)}
        onCancelName={cancelName}
        onBoardMenu={openBoardMenu}
        onBoardPointerDown={(e, board) => startDrag(e, { type: "board", id: board.id, label: board.name })}
      />
      <div className={showPreview ? "content with-preview" : "content"}>
        <div className="results">{results}</div>
        {showPreview && <Preview id={selectedHit.id} version={version} query={state.text} />}
      </div>
      <NoticeBar notice={notice} />
      <Hints typing={state.text !== ""} previewOpen={previewOpen} inBoard={typeof state.view === "number"} />
      {menu && <Menu {...menu} onClose={closeMenu} />}
      {drag && (
        <div className="drag-ghost" style={{ left: drag.x, top: drag.y }} aria-hidden="true">
          {drag.item.label}
        </div>
      )}
    </div>
  );
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
