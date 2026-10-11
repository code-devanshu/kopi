"use client";

import { useEffect } from "react";

import { trackEvent } from "@/lib/analytics";

/**
 * One delegated listener, so server components can be tracked without becoming
 * client components. Mark an element with `data-track="event_name"`; every
 * `data-track-foo="bar"` on it becomes the event param `foo: "bar"`.
 */
export function ClickTracker() {
  useEffect(() => {
    function onClick(event: MouseEvent) {
      const el = event.target instanceof Element ? event.target.closest<HTMLElement>("[data-track]") : null;
      if (!el?.dataset.track) return;
      const params: Record<string, string> = {};
      for (const [key, value] of Object.entries(el.dataset)) {
        if (key === "track" || !key.startsWith("track") || value === undefined) continue;
        const name = key.slice("track".length);
        params[name.charAt(0).toLowerCase() + name.slice(1)] = value;
      }
      trackEvent(el.dataset.track, params);
    }
    document.addEventListener("click", onClick);
    return () => document.removeEventListener("click", onClick);
  }, []);

  return null;
}
