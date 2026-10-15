// Pasting and copying from the panel. The Rust side writes the clipboard, hides the panel and
// posts ⌘V. This decides what the panel shows meanwhile, and when it closes.
import type { PasteOutcome } from "../../lib/ipc";
import { keys } from "../../lib/platform";

/** A short message over the bottom of the panel. */
export type Notice = {
  message: string;
  detail?: string;
  tone: "info" | "error";
  /** A button in the message. */
  action?: { label: string; run: () => void };
};

export type PasterDeps = {
  paste: (id: number, plain: boolean) => Promise<PasteOutcome>;
  copy: (id: number, plain: boolean) => Promise<unknown>;
  hide: () => void;
  notify: (notice: Notice | null) => void;
  /** Shows where to allow Kopi under Accessibility, so it can paste. */
  openAccessibility: () => void;
};

/** How long "Copied" shows before the panel closes, so there's time to read it and press its
 * button. */
export const COPIED_CLOSE_MS = 6000;
/** After ⌘C the message is only a confirmation. */
export const COPY_CLOSE_MS = 700;

export class Paster {
  #deps: PasterDeps;
  /** A paste or copy is on its way, so another Return doesn't send a second one. */
  #busy = false;
  #closeTimer: ReturnType<typeof setTimeout> | undefined;

  constructor(deps: PasterDeps) {
    this.#deps = deps;
  }

  /** Whether the panel is about to close after a copy. ⌘V then closes it at once. */
  get closing(): boolean {
    return this.#closeTimer !== undefined;
  }

  async paste(id: number, plain: boolean) {
    await this.#run(async () => {
      // "pasted" means the panel is already hidden.
      if ((await this.#deps.paste(id, plain)) === "copied") {
        this.#closeAfter(COPIED_CLOSE_MS, {
          message: `Copied. Press ${keys("⌘V")} to paste.`,
          detail:
            "To have Kopi paste for you, turn on Kopi in System Settings → Privacy & Security → Accessibility. Not listed? Add it with +. Already on? Remove it with −, then add it again.",
          tone: "info",
          action: { label: "Open Accessibility Settings", run: () => this.#openAccessibility() },
        });
      }
    }, "Couldn't paste");
  }

  async copy(id: number, plain: boolean) {
    await this.#run(async () => {
      await this.#deps.copy(id, plain);
      this.#closeAfter(COPY_CLOSE_MS, { message: "Copied.", tone: "info" });
    }, "Couldn't copy");
  }

  /** Closes the panel now, instead of after the message. */
  closeNow() {
    this.reset();
    this.#deps.hide();
  }

  #openAccessibility() {
    this.closeNow();
    this.#deps.openAccessibility();
  }

  /** Drops the message and any planned close: the panel was shown or hidden. */
  reset() {
    clearTimeout(this.#closeTimer);
    this.#closeTimer = undefined;
    this.#deps.notify(null);
  }

  async #run(f: () => Promise<void>, failure: string) {
    if (this.#busy) return;
    this.#busy = true;
    this.reset();
    try {
      await f();
    } catch (e) {
      this.#deps.notify({ message: `${failure}.`, detail: String(e), tone: "error" });
    } finally {
      this.#busy = false;
    }
  }

  #closeAfter(ms: number, notice: Notice) {
    this.#deps.notify(notice);
    this.#closeTimer = setTimeout(() => this.closeNow(), ms);
  }
}
