/** GA4 measurement ID. Override with NEXT_PUBLIC_GA_ID. */
export const GA_ID = process.env.NEXT_PUBLIC_GA_ID || "G-Y2J68GL859";

type EventParams = Record<string, string | number | boolean | undefined>;

declare global {
  interface Window {
    gtag?: (...args: unknown[]) => void;
  }
}

/**
 * Sends a GA4 event. Does nothing when gtag isn't loaded: in development, or when
 * a blocker stops it. Never pass personal data (emails, names, free text) as params.
 */
export function trackEvent(name: string, params?: EventParams) {
  if (typeof window === "undefined" || typeof window.gtag !== "function") return;
  window.gtag("event", name, params);
}
