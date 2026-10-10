import { Command } from "lucide-react";
import type { ComponentType, SVGProps } from "react";

import { WindowsIcon } from "@/components/brand/brand-icons";

const platforms: Array<{ id: string; label: string; icon: ComponentType<SVGProps<SVGSVGElement>> }> = [
  { id: "macos", label: "macOS", icon: Command },
  { id: "windows", label: "Windows", icon: WindowsIcon },
];

/** Visible but disabled until release. aria-disabled keeps them focusable so the tooltip is reachable. */
export function DownloadButtons() {
  return (
    <div className="flex flex-wrap gap-3">
      {platforms.map(({ id, label, icon: Icon }) => (
        <div key={id} className="group relative">
          <button
            type="button"
            aria-disabled="true"
            aria-describedby={`download-tip-${id}`}
            data-track="download_click"
            data-track-platform={id}
            data-track-available="false"
            className="inline-flex h-11 cursor-not-allowed items-center gap-2 rounded-full border border-dashed border-border bg-surface/60 px-4 text-sm text-muted-foreground"
          >
            <Icon className="size-4" aria-hidden="true" />
            <span className="font-medium text-foreground">{label}</span>
            <span>Coming soon</span>
          </button>
          <span
            id={`download-tip-${id}`}
            role="tooltip"
            className="pointer-events-none absolute bottom-full left-1/2 z-10 mb-2 w-max max-w-56 -translate-x-1/2 translate-y-1 rounded-lg bg-foreground px-3 py-2 text-center text-xs text-background opacity-0 shadow-soft transition duration-150 group-focus-within:translate-y-0 group-focus-within:opacity-100 group-hover:translate-y-0 group-hover:opacity-100"
          >
            Not available yet. Join the waitlist and we&apos;ll email you when it is.
          </span>
        </div>
      ))}
    </div>
  );
}
