import { siteConfig } from "@/lib/site";

const link = "font-medium text-foreground underline decoration-border underline-offset-4 hover:decoration-foreground";

/** What each download needs. The download dialog explains what the OS says the first time. */
export function PlatformNote() {
  return (
    <p className="text-sm text-muted-foreground">
      macOS 14 or later · Windows 10 and 11, 64-bit ·{" "}
      <a
        href={siteConfig.links.install}
        data-track="install_guide_click"
        data-track-location="hero"
        target="_blank"
        rel="noreferrer"
        className={link}
      >
        Install guide
      </a>
    </p>
  );
}
