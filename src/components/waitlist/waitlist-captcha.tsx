"use client";

import HCaptcha from "@hcaptcha/react-hcaptcha";
import { useSyncExternalStore } from "react";

import { cn } from "@/lib/cn";

/**
 * Web3Forms' shared hCaptcha site key for the free plan. Web3Forms verifies the
 * token itself, so there's no secret key to keep anywhere.
 */
const HCAPTCHA_SITEKEY = "50b2fe65-b00b-4b9e-ad62-3ba471098be2";

// Follow the site's light/dark toggle, which sets data-theme on <html>.
function subscribeToTheme(onChange: () => void) {
  const observer = new MutationObserver(onChange);
  observer.observe(document.documentElement, { attributes: true, attributeFilter: ["data-theme"] });
  return () => observer.disconnect();
}
const getTheme = (): "light" | "dark" => (document.documentElement.getAttribute("data-theme") === "dark" ? "dark" : "light");

// The full widget is 303px wide, wider than the form on small phones, so use the compact one there.
const WIDE_QUERY = "(min-width: 400px)";
function subscribeToWidth(onChange: () => void) {
  const query = window.matchMedia(WIDE_QUERY);
  query.addEventListener("change", onChange);
  return () => query.removeEventListener("change", onChange);
}
const getIsWide = () => window.matchMedia(WIDE_QUERY).matches;

export function WaitlistCaptcha({
  onToken,
}: {
  /** Called with a fresh token, or null when the token expires or the widget errors. */
  onToken: (token: string | null) => void;
}) {
  const theme = useSyncExternalStore(subscribeToTheme, getTheme, () => "light" as const);
  const isWide = useSyncExternalStore(subscribeToWidth, getIsWide, () => true);

  return (
    // Reserve the widget's height so the form doesn't jump when it loads.
    <div className={cn("flex justify-center", isWide ? "min-h-[78px]" : "min-h-[144px]")}>
      <HCaptcha
        sitekey={HCAPTCHA_SITEKEY}
        reCaptchaCompat={false}
        theme={theme}
        size={isWide ? "normal" : "compact"}
        onVerify={(token) => onToken(token)}
        onExpire={() => onToken(null)}
        onError={() => onToken(null)}
      />
    </div>
  );
}
