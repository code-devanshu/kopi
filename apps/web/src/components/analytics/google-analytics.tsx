import Script from "next/script";

import { GA_ID } from "@/lib/analytics";

/**
 * GA4 base tag. Production builds only. On localhost (a `next start` preview) the
 * official ga-disable flag stops any hits, so local testing stays out of the reports.
 * GA's own page_view covers navigation, client-side page changes included.
 */
export function GoogleAnalytics() {
  if (process.env.NODE_ENV !== "production") return null;

  const id = JSON.stringify(GA_ID);
  return (
    <>
      <Script src={`https://www.googletagmanager.com/gtag/js?id=${encodeURIComponent(GA_ID)}`} strategy="afterInteractive" />
      <Script id="ga-init" strategy="afterInteractive">
        {`if (/^(localhost|127\\.0\\.0\\.1|\\[::1\\])$/.test(location.hostname)) window["ga-disable-" + ${id}] = true;
window.dataLayer = window.dataLayer || [];
function gtag(){dataLayer.push(arguments);}
gtag("js", new Date());
gtag("config", ${id});`}
      </Script>
    </>
  );
}
