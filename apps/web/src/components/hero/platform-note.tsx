import { siteConfig } from "@/lib/site";

const link = "font-medium text-foreground underline decoration-border underline-offset-4 hover:decoration-foreground";

/** What each download needs, and what the OS says the first time. */
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
        Windows 10 and 11, on 64-bit PCs. The installer isn&apos;t signed yet, so Windows may warn you the first
        time.{" "}
        <a href={siteConfig.links.installWindows} data-track="install_guide_click" data-track-location="hero_windows" target="_blank" rel="noreferrer" className={link}>
          Here&apos;s what to do
        </a>.
      </p>
    </div>
  );
}
