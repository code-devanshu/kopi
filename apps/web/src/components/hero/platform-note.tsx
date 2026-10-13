import { siteConfig } from "@/lib/site";

const link = "font-medium text-foreground underline decoration-border underline-offset-4 hover:decoration-foreground";

/** What the download is, and where Windows stands. */
export function PlatformNote() {
  return (
    <div className="max-w-xl space-y-1.5 text-sm leading-relaxed text-muted-foreground">
      <p>
        Free alpha for macOS 14 or later, on Apple silicon and Intel. Apple doesn&apos;t notarize it, so macOS
        blocks the download the first time.{" "}
        <a href={siteConfig.links.install} data-track="install_guide_click" data-track-location="hero" target="_blank" rel="noreferrer" className={link}>
          Here&apos;s how to open it
        </a>.
      </p>
      <p>
        Windows is next.{" "}
        <a href="#waitlist" data-track="waitlist_cta_click" data-track-location="hero" className={link}>
          Join the waitlist
        </a>{" "}
        to hear when it&apos;s ready.
      </p>
    </div>
  );
}
