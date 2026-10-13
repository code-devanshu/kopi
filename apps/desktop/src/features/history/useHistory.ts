import { useSyncExternalStore } from "react";
import type { History, HistoryState } from "./history";

/** The history's state, re-rendering on every change. */
export function useHistory(history: History): HistoryState {
  return useSyncExternalStore(history.subscribe, history.getSnapshot);
}
