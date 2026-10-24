import type { Metadata } from "next";

import { ProductHuntLaunch } from "@/components/brand/product-hunt-badge";
import { DownloadOptions } from "@/components/download/download-options";
import { InstallCommand } from "@/components/hero/install-command";
import { Container } from "@/components/ui/section";
import { siteConfig } from "@/lib/site";

export const metadata: Metadata = {
  title: "Download",
  description: "Download Kopi, the free and open-source clipboard manager, for macOS 14 or later and Windows 10 and 11.",
  alternates: { canonical: "/download" },
};

const link = "font-medium text-foreground underline decoration-border underline-offset-4 hover:decoration-foreground";

export default function DownloadPage() {
  return (
    <section aria-labelledby="download-title" className="relative overflow-hidden">
      <div
        aria-hidden="true"
        className="pointer-events-none absolute inset-0 bg-[radial-gradient(55%_45%_at_50%_0%,var(--accent-soft),transparent_75%)]"
      />
      <Container className="relative max-w-4xl py-16 text-center sm:py-24">
        <h1 id="download-title" className="text-5xl leading-[1.02] font-semibold tracking-[-0.035em] sm:text-6xl">
          Download Kopi
        </h1>
        <p className="mx-auto mt-5 max-w-xl text-lg text-muted-foreground">
          Free and open source. Kopi is in alpha, and updates itself after the first install.
        </p>

        <ProductHuntLaunch location="download_page" className="mt-8" />

        <div className="mt-10">
          <DownloadOptions />
        </div>

        <div className="mt-8 space-y-3">
          <InstallCommand location="download_page" />
          <p className="text-sm text-muted-foreground">
            <a
              href={siteConfig.links.install}
              data-track="install_guide_click"
              data-track-location="download_page"
              target="_blank"
              rel="noreferrer"
              className={link}
            >
              Install guide
            </a>{" "}
            ·{" "}
            <a
              href={siteConfig.links.releases}
              data-track="releases_click"
              data-track-location="download_page"
              target="_blank"
              rel="noreferrer"
              className={link}
            >
              Every release on GitHub
            </a>
          </p>
        </div>
      </Container>
    </section>
  );
}
