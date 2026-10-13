"use client";

import { Check, Copy } from "lucide-react";
import { useEffect, useState } from "react";

import { trackEvent } from "@/lib/analytics";
import { siteConfig } from "@/lib/site";

/** The one-line install. macOS doesn't block Kopi installed this way, unlike the .dmg from a browser. */
export function InstallCommand() {
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    if (!copied) return;
    const timer = setTimeout(() => setCopied(false), 2000);
    return () => clearTimeout(timer);
  }, [copied]);

  async function copy() {
    try {
      await navigator.clipboard.writeText(siteConfig.installCommand);
      setCopied(true);
    } catch {
      // The command stays on screen to copy by hand.
    }
  }

  return (
    <div className="max-w-xl">
      <p className="text-sm text-muted-foreground">Or install from Terminal, and macOS won&apos;t block it the first time:</p>
      <div className="mt-2 flex items-center gap-2 rounded-xl border border-border bg-surface py-1.5 pr-1.5 pl-4 shadow-soft">
        {/* Copying the command by hand counts too, not only the button. */}
        <code
          onCopy={() => trackEvent("install_command_copy", { location: "hero", method: "manual" })}
          className="min-w-0 flex-1 overflow-x-auto font-mono text-sm whitespace-nowrap text-foreground"
        >
          {siteConfig.installCommand}
        </code>
        <button
          type="button"
          onClick={copy}
          data-track="install_command_copy" data-track-location="hero" data-track-method="button"
          aria-label={copied ? "Copied" : "Copy the install command"}
          className="inline-flex size-8 shrink-0 items-center justify-center rounded-lg text-muted-foreground transition-colors hover:bg-surface-muted hover:text-foreground"
        >
          {copied ? <Check className="size-4" aria-hidden="true" /> : <Copy className="size-4" aria-hidden="true" />}
        </button>
      </div>
      <span className="sr-only" aria-live="polite">
        {copied ? "Copied" : ""}
      </span>
    </div>
  );
}
