"use client";

import { AppleIcon, WindowsIcon } from "@/components/brand/brand-icons";
import { DownloadButton, useDetectedPlatform } from "@/components/download/download-button";
import { buttonStyles } from "@/components/ui/button-styles";

/**
 * Both downloads, with the visitor's OS as the main button. The order stays put, so nothing
 * moves when the OS is detected; until then, and on phones, macOS leads.
 */
export function HeroDownloads() {
  const windows = useDetectedPlatform() === "windows";

  return (
    <div className="flex flex-col gap-3 sm:flex-row">
      <DownloadButton
        platform="macos"
        location="hero"
        className={buttonStyles({ variant: windows ? "secondary" : "primary", size: "lg" })}
      >
        <AppleIcon className="size-4" />
        Download for macOS
      </DownloadButton>
      <DownloadButton
        platform="windows"
        location="hero"
        className={buttonStyles({ variant: windows ? "primary" : "secondary", size: "lg" })}
      >
        <WindowsIcon className="size-4" />
        Download for Windows
      </DownloadButton>
    </div>
  );
}
