"use client";

import { AppleIcon, WindowsIcon } from "@/components/brand/brand-icons";
import { DownloadButton, useDetectedPlatform } from "@/components/download/download-button";
import { buttonStyles } from "@/components/ui/button-styles";
import type { Platform } from "@/lib/downloads";
import { cn } from "@/lib/cn";

const options = [
  {
    platform: "macos",
    name: "macOS",
    Icon: AppleIcon,
    needs: "macOS 14 Sonoma or later. One app for Apple silicon and Intel Macs.",
    file: ".dmg",
  },
  {
    platform: "windows",
    name: "Windows",
    Icon: WindowsIcon,
    needs: "Windows 10 or 11, 64-bit.",
    file: "setup .exe",
  },
] as const satisfies readonly { platform: Platform; [key: string]: unknown }[];

/** A card per OS, with the visitor's OS as the main button. macOS leads until it's known. */
export function DownloadOptions() {
  const windows = useDetectedPlatform() === "windows";

  return (
    <div className="grid gap-4 sm:grid-cols-2">
      {options.map(({ platform, name, Icon, needs, file }) => {
        const main = (platform === "windows") === windows;
        return (
          <div
            key={platform}
            className={cn(
              "flex flex-col rounded-3xl border bg-surface p-6 text-left shadow-soft sm:p-8",
              main ? "border-accent/60" : "border-border",
            )}
          >
            <Icon className="size-7" aria-hidden="true" />
            <h2 className="mt-5 text-2xl font-semibold">{name}</h2>
            <p className="mt-2 flex-1 text-muted-foreground">{needs}</p>
            <DownloadButton
              platform={platform}
              location="download_page"
              className={buttonStyles({ variant: main ? "primary" : "secondary", size: "lg", className: "mt-6 w-full" })}
            >
              <Icon className="size-4" />
              Download for {name}
            </DownloadButton>
            <p className="mt-3 text-center text-xs text-muted-foreground">The newest alpha, as a {file}</p>
          </div>
        );
      })}
    </div>
  );
}
