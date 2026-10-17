"use client";

import { X } from "lucide-react";
import { useId, useRef, useSyncExternalStore, type ReactNode } from "react";

import { buttonStyles } from "@/components/ui/button-styles";
import type { Platform } from "@/lib/downloads";
import { siteConfig } from "@/lib/site";

type Props = {
  /** `auto` picks the visitor's OS, and scrolls to the hero's buttons on any other. */
  platform: Platform | "auto";
  location: string;
  className?: string;
  children: ReactNode;
};

const noop = () => () => {};

function detectPlatform(): Platform | null {
  const ua = navigator.userAgent;
  if (/Windows/.test(ua)) return "windows";
  if (/Macintosh/.test(ua) && !/iPhone|iPad/.test(ua)) return "macos";
  return null;
}

/**
 * Downloads the newest release's file without leaving the page, then shows how to install it.
 * GitHub's releases page lists eight files and loses people who don't know which one they need.
 */
/** The visitor's OS, or null before hydration and on phones and Linux. */
export function useDetectedPlatform(): Platform | null {
  return useSyncExternalStore(noop, detectPlatform, () => null);
}

export function DownloadButton({ platform, location, className, children }: Props) {
  const detected = useDetectedPlatform();
  const target = platform === "auto" ? detected : platform;
  const dialog = useRef<HTMLDialogElement>(null);
  const titleId = useId();

  return (
    <>
      <a
        href={target ? `/download/${target}` : "#top"}
        data-track="download_click"
        data-track-platform={target ?? "unknown"}
        data-track-location={location}
        // The link downloads the file; the page stays and shows what to do with it.
        onClick={() => target && dialog.current?.showModal()}
        className={className}
      >
        {children}
      </a>
      {target ? (
        <dialog
          ref={dialog}
          aria-labelledby={titleId}
          // A click on the backdrop lands on the dialog itself.
          onClick={(event) => event.target === event.currentTarget && dialog.current?.close()}
          className="m-auto max-h-[calc(100dvh-2rem)] w-[min(34rem,calc(100%-2rem))] rounded-3xl border border-border bg-surface p-0 text-foreground shadow-lifted backdrop:bg-black/50 backdrop:backdrop-blur-sm"
        >
          <div className="p-6 sm:p-8">
            <div className="flex items-start justify-between gap-4">
              <div>
                <h2 id={titleId} className="text-xl font-semibold">
                  Kopi is downloading
                </h2>
                <p className="mt-1 text-sm text-muted-foreground">
                  {target === "macos" ? "Here's how to install it on your Mac." : "Here's how to install it on your PC."}
                </p>
              </div>
              <button
                type="button"
                onClick={() => dialog.current?.close()}
                aria-label="Close"
                className="-mt-1 -mr-2 inline-flex size-9 shrink-0 items-center justify-center rounded-full text-muted-foreground hover:bg-surface-muted hover:text-foreground"
              >
                <X className="size-5" aria-hidden="true" />
              </button>
            </div>

            {target === "macos" ? <MacSteps /> : <WindowsSteps />}

            <p className="mt-6 text-sm text-muted-foreground">
              Didn&apos;t start?{" "}
              <a href={`/download/${target}`} data-track="download_retry_click" data-track-platform={target} className={link}>
                Download again
              </a>
              . Stuck?{" "}
              <a
                href={target === "macos" ? siteConfig.links.install : siteConfig.links.installWindows}
                data-track="install_guide_click"
                data-track-location="download_dialog"
                target="_blank"
                rel="noreferrer"
                className={link}
              >
                Read the full guide
              </a>
              .
            </p>

            <button type="button" onClick={() => dialog.current?.close()} className={buttonStyles({ className: "mt-6 w-full" })}>
              Got it
            </button>
          </div>
        </dialog>
      ) : null}
    </>
  );
}

const link = "font-medium text-foreground underline decoration-border underline-offset-4 hover:decoration-foreground";
const kbd = "rounded-md border border-border bg-surface-muted px-1.5 py-0.5 font-mono text-[0.85em]";

function Steps({ children }: { children: ReactNode }) {
  return (
    <ol className="mt-6 space-y-4 text-[0.95rem] leading-relaxed [counter-reset:step]">
      {children}
    </ol>
  );
}

function Step({ children }: { children: ReactNode }) {
  return (
    <li className="relative pl-10 [counter-increment:step] before:absolute before:top-0 before:left-0 before:flex before:size-7 before:items-center before:justify-center before:rounded-full before:bg-accent-soft before:text-sm before:font-semibold before:text-accent-text before:content-[counter(step)]">
      {children}
    </li>
  );
}

function MacSteps() {
  return (
    <Steps>
      <Step>
        Open the <strong>Kopi .dmg</strong> in your Downloads folder, and drag <strong>Kopi</strong> onto{" "}
        <strong>Applications</strong>.
      </Step>
      <Step>
        Open Kopi from Applications. Apple hasn&apos;t checked Kopi, so macOS says it wasn&apos;t opened. Click{" "}
        <strong>Done</strong>.
      </Step>
      <Step>
        Open <strong>System Settings → Privacy &amp; Security</strong>, scroll down and click{" "}
        <strong>Open Anyway</strong> next to &ldquo;Kopi&rdquo; was blocked. Enter your password, then click{" "}
        <strong>Open Anyway</strong> again.
        <span className="mt-1 block text-sm text-muted-foreground">
          On macOS 14 Sonoma, Control-click Kopi in Applications and choose <strong>Open</strong> instead.
        </span>
      </Step>
      <Step>
        Kopi lives in the menu bar, not the Dock, and walks you through setup. Then press{" "}
        <kbd className={kbd}>⇧⌘V</kbd> in any app.
      </Step>
    </Steps>
  );
}

function WindowsSteps() {
  return (
    <Steps>
      <Step>
        Open the <strong>Kopi setup .exe</strong> in your Downloads folder.
      </Step>
      <Step>
        If Windows says <strong>Windows protected your PC</strong>, choose <strong>More info</strong>, then{" "}
        <strong>Run anyway</strong>. The installer isn&apos;t signed yet.
      </Step>
      <Step>
        Kopi starts when the installer finishes. Its icon is at the right of the taskbar, maybe behind the{" "}
        <strong>^</strong> arrow.
      </Step>
      <Step>
        Press <kbd className={kbd}>Ctrl+Shift+V</kbd> in any app to open Kopi.
      </Step>
    </Steps>
  );
}
