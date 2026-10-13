import { useCallback, useEffect, useRef, useState, type KeyboardEvent } from "react";
import { ClipList } from "../../features/history/ClipList";
import { History } from "../../features/history/history";
import { actionFor } from "../../features/history/keys";
import { Empty, LoadError, LoadingRows } from "../../features/history/States";
import { useHistory } from "../../features/history/useHistory";
import { Preview } from "../../features/preview/Preview";
import { SearchBar } from "../../features/search/SearchBar";
import { commands, events, unwrap, type SourceApp } from "../../lib/ipc";
import { Hints } from "./Hints";

const LIST_ID = "clip-list";
/** Whether the preview is open, remembered between launches. */
const PREVIEW_KEY = "kopi.preview";
/** How often the "5m ago" times are brought up to date while the panel is open. */
const CLOCK_MS = 30_000;

export function Panel() {
  const [history] = useState(() => new History((query) => unwrap(commands.searchClips(query))));
  const state = useHistory(history);
  const inputRef = useRef<HTMLInputElement>(null);
  const pageSize = useRef(8);
  const [apps, setApps] = useState<SourceApp[]>([]);
  const [now, setNow] = useState(() => Date.now());
  const [previewOpen, setPreviewOpen] = useState(() => load(PREVIEW_KEY) !== "closed");
  // Bumped when a clip changes, so the preview fetches it again.
  const [version, setVersion] = useState(0);
  // Per content hash, how many times its thumbnail was written. Rows load it again on a bump.
  const [thumbnails, setThumbnails] = useState<ReadonlyMap<string, number>>(new Map());

  useEffect(() => {
    const loadApps = () => unwrap(commands.sourceApps()).then(setApps, () => {});
    void history.reset();
    loadApps();

    const shown = () => {
      setNow(Date.now());
      void history.reset();
      loadApps();
      // The webview has focus a frame after the panel is shown.
      requestAnimationFrame(() => inputRef.current?.focus());
    };
    const changed = () => {
      history.refreshSoon();
      setVersion((v) => v + 1);
    };
    const listeners = [
      events.panelShown.listen(shown),
      events.clipAdded.listen(() => history.refreshSoon()),
      events.clipUpdated.listen(changed),
      events.clipRemoved.listen(changed),
      events.thumbnailReady.listen(({ payload: { contentHash } }) =>
        setThumbnails((m) => new Map(m).set(contentHash, (m.get(contentHash) ?? 0) + 1)),
      ),
    ];
    const clock = setInterval(() => setNow(Date.now()), CLOCK_MS);
    return () => {
      clearInterval(clock);
      listeners.forEach((unlisten) => void unlisten.then((stop) => stop()));
    };
  }, [history]);

  useEffect(() => save(PREVIEW_KEY, previewOpen ? "open" : "closed"), [previewOpen]);

  const select = useCallback((index: number) => history.select(index), [history]);
  // Pasting arrives with the paste engine (#14). Until then, picking a clip selects it.
  const pick = useCallback((index: number) => history.select(index), [history]);
  const loadMore = useCallback(() => void history.loadMore(), [history]);
  const setPageSize = useCallback((rows: number) => {
    pageSize.current = rows;
  }, []);

  function onKeyDown(e: KeyboardEvent) {
    const action = actionFor(e.nativeEvent, state.text !== "");
    if (!action) return;
    // A focused filter keeps its own arrow keys.
    if (e.target instanceof HTMLSelectElement && action.type !== "hide") return;
    e.preventDefault();
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
        return pick(state.selected);
      case "quickPaste":
        if (action.index < state.hits.length) pick(action.index);
        return;
      case "delete": {
        const hit = history.selectedHit;
        // The list updates from the clip:removed event.
        if (hit) unwrap(commands.deleteClips([hit.id])).catch(() => history.refresh());
        return;
      }
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
        searchKey={`${state.text}\n${state.kind}\n${state.appId}`}
        hits={state.hits}
        selected={state.selected}
        now={now}
        thumbnailAttempts={thumbnails}
        onSelect={select}
        onPick={pick}
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
      <div className={showPreview ? "content with-preview" : "content"}>
        <div className="results">{results}</div>
        {showPreview && <Preview id={selectedHit.id} version={version} query={state.text} />}
      </div>
      <Hints typing={state.text !== ""} previewOpen={previewOpen} />
    </div>
  );
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
