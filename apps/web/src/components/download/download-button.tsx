"use client";

import Link from "next/link";
import { useSyncExternalStore, type ReactNode } from "react";

import type { Platform } from "@/lib/downloads";

type Props = {
  /** `auto` picks the visitor's OS, and goes to /download on any other. */
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

/** The visitor's OS, or null before hydration and on phones and Linux. */
export function useDetectedPlatform(): Platform | null {
  return useSyncExternalStore(noop, detectPlatform, () => null);
}

/**
 * Goes to the page that downloads the newest release's file and shows how to install it.
 * GitHub's releases page lists eight files and loses people who don't know which one they need.
 */
export function DownloadButton({ platform, location, className, children }: Props) {
  const detected = useDetectedPlatform();
  const target = platform === "auto" ? detected : platform;

  return (
    <Link
      href={target ? `/download/${target}/thanks` : "/download"}
      // The page after a download reads its platform from the URL, outside the shared shell.
      prefetch={target ? true : undefined}
      data-track="download_click"
      data-track-platform={target ?? "unknown"}
      data-track-location={location}
      className={className}
    >
      {children}
    </Link>
  );
}
