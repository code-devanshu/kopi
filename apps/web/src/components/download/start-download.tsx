"use client";

import { useEffect, useRef } from "react";

import type { Platform } from "@/lib/downloads";

/**
 * Starts the download when the page opens. GitHub sends the file as an attachment, so the
 * browser downloads it and stays here with the steps.
 */
export function StartDownload({ platform }: { platform: Platform }) {
  // Strict Mode runs effects twice in development; one download is enough.
  const started = useRef(false);

  useEffect(() => {
    if (started.current) return;
    started.current = true;
    // A route handler that answers with the file, not a page, so not the router.
    // eslint-disable-next-line @next/next/no-location-assign-relative-destination
    window.location.assign(`/download/${platform}`);
  }, [platform]);

  return null;
}
