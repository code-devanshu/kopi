import type { ReactNode } from "react";

import type { Platform } from "@/lib/downloads";

/** How to install the download, as the OS shows it the first time. */
export function InstallSteps({ platform }: { platform: Platform }) {
  return platform === "macos" ? <MacSteps /> : <WindowsSteps />;
}

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
