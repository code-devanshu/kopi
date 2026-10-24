import type { Metadata } from "next";
import { notFound } from "next/navigation";

import { ProductHuntBadge } from "@/components/brand/product-hunt-badge";
import { InstallSteps } from "@/components/download/install-steps";
import { StartDownload } from "@/components/download/start-download";
import { Container } from "@/components/ui/section";
import { isPlatform, platforms } from "@/lib/downloads";
import { siteConfig } from "@/lib/site";

export const metadata: Metadata = {
  title: "Kopi is downloading",
  // A step after a click, not a page to land on from a search.
  robots: { index: false },
};

// The whole page is about one platform, and each is prerendered, so there's no shell to show
// first. The download buttons prefetch it instead.
export const instant = false;

export function generateStaticParams() {
  return platforms.map((platform) => ({ platform }));
}

const link = "font-medium text-foreground underline decoration-border underline-offset-4 hover:decoration-foreground";

/** After a download button: starts the download, then shows how to install it. */
export default async function DownloadThanks({ params }: PageProps<"/download/[platform]/thanks">) {
  const { platform } = await params;
  if (!isPlatform(platform)) notFound();
  const mac = platform === "macos";

  return (
    <Container className="max-w-2xl py-16 sm:py-24">
      <StartDownload platform={platform} />

      <h1 className="text-4xl leading-[1.05] font-semibold tracking-[-0.035em] sm:text-5xl">Kopi is downloading</h1>
      <p className="mt-4 text-lg text-muted-foreground">
        {mac ? "Here's how to install it on your Mac." : "Here's how to install it on your PC."}
      </p>

      <div className="mt-8 rounded-3xl border border-border bg-surface p-6 shadow-soft sm:p-8 [&>ol]:mt-0">
        <InstallSteps platform={platform} />
        <p className="mt-6 text-sm text-muted-foreground">
          Didn&apos;t start?{" "}
          <a href={`/download/${platform}`} data-track="download_retry_click" data-track-platform={platform} className={link}>
            Download again
          </a>
          . Stuck?{" "}
          <a
            href={mac ? siteConfig.links.install : siteConfig.links.installWindows}
            data-track="install_guide_click"
            data-track-location="download_thanks"
            target="_blank"
            rel="noreferrer"
            className={link}
          >
            Read the full guide
          </a>
          .
        </p>
      </div>

      <div className="mt-12 flex flex-col items-start gap-4 border-t border-border pt-10 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <h2 className="text-xl font-semibold">Tried it for a few days?</h2>
          <p className="mt-1 text-muted-foreground">Tell other people what you think on Product Hunt.</p>
        </div>
        <ProductHuntBadge variant="review" location="download_thanks" />
      </div>
    </Container>
  );
}
