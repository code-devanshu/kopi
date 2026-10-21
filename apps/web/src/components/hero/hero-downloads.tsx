"use client";

import { AppleIcon, WindowsIcon } from "@/components/brand/brand-icons";
import { DownloadButton, useDetectedPlatform } from "@/components/download/download-button";
import { buttonStyles } from "@/components/ui/button-styles";
import { cn } from "@/lib/cn";

/**
 * Both downloads, with the visitor's OS as the main button. The order stays put, so nothing
 * moves when the OS is detected; until then, and on phones, macOS leads.
 */
export function HeroDownloads({ location = "hero", className }: { location?: string; className?: string }) {
  const windows = useDetectedPlatform() === "windows";

  return (
    <div className={cn("flex flex-col gap-3 sm:flex-row", className)}>
      <DownloadButton
        platform="macos"
        location={location}
        className={buttonStyles({ variant: windows ? "secondary" : "primary", size: "lg" })}
      >
        <AppleIcon className="size-4" />
        Download for macOS
      </DownloadButton>
      <DownloadButton
        platform="windows"
        location={location}
        className={buttonStyles({ variant: windows ? "primary" : "secondary", size: "lg" })}
      >
        <WindowsIcon className="size-4" />
        Download for Windows
      </DownloadButton>
    </div>
  );
}
