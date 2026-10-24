import { GitHubIcon } from "@/components/brand/brand-icons";
import { Logo } from "@/components/brand/logo";
import { ProductHuntBadge, SHOW_PH_LAUNCH_BADGE_IN_HERO } from "@/components/brand/product-hunt-badge";
import { Container } from "@/components/ui/section";
import { siteConfig } from "@/lib/site";

const links = [
  { label: "GitHub", href: siteConfig.links.github, external: true, track: { "data-track": "github_click" } },
  {
    label: "Privacy",
    href: siteConfig.links.privacy,
    external: false,
    track: { "data-track": "nav_click", "data-track-section": "privacy" },
  },
  { label: "Contact", href: siteConfig.links.contact, external: false, track: { "data-track": "contact_click" } },
];

export function SiteFooter() {
  return (
    <footer className="border-t border-border">
      <Container className="flex flex-col gap-10 py-12 md:flex-row md:items-start md:justify-between">
        <div>
          <Logo />
          <p className="mt-4 max-w-xs text-sm leading-relaxed text-muted-foreground">
            An open-source clipboard manager. The alpha is out for macOS and Windows.
          </p>
          <div className="mt-5 flex flex-wrap items-center gap-x-4 gap-y-3">
            <a
              href={siteConfig.links.github}
              data-track="github_click"
              data-track-location="footer"
              target="_blank"
              rel="noreferrer"
              aria-label="Kopi on GitHub"
              className="inline-flex size-9 items-center justify-center rounded-full border border-border text-muted-foreground transition-colors hover:text-foreground"
            >
              <GitHubIcon className="size-4" />
            </a>
            {/* TryMacApps lists Kopi for free while this badge is on the homepage. Their checker
                looks for this exact link and image, so keep it a plain <img>. The badge is dark gray
                and they have no dark version, so it's inverted in dark mode. */}
            <a href="https://www.trymacapps.com" target="_blank" rel="noopener">
              {/* eslint-disable-next-line @next/next/no-img-element */}
              <img
                src="https://www.trymacapps.com/badge.png"
                alt="Featured on TryMacApps"
                width="200"
                height="67"
                className="dark:invert"
              />
            </a>
            {/* Findly.tools checks the homepage for this link and the light badge, so keep both
                exact and plain <img>s. The dark badge is lazy so light mode doesn't fetch it. */}
            <a href="https://findly.tools/kopi?utm_source=kopi" target="_blank" rel="noopener noreferrer">
              {/* eslint-disable-next-line @next/next/no-img-element */}
              <img
                src="https://findly.tools/badges/findly-tools-badge-light.svg"
                alt="Featured on Findly.tools"
                width="175"
                height="55"
                className="dark:hidden"
              />
              {/* eslint-disable-next-line @next/next/no-img-element */}
              <img
                src="https://findly.tools/badges/findly-tools-badge-dark.svg"
                alt="Featured on Findly.tools"
                width="175"
                height="55"
                loading="lazy"
                className="hidden dark:block"
              />
            </a>
            {/* After launch day, the launch badge leaves the hero for here. */}
            {SHOW_PH_LAUNCH_BADGE_IN_HERO ? null : <ProductHuntBadge variant="launch" location="footer" />}
            <ProductHuntBadge variant="follow" location="footer" />
          </div>
        </div>

        <nav aria-label="Footer">
          <ul className="grid grid-cols-2 gap-x-12 gap-y-3 text-sm">
            {links.map((link) => (
              <li key={link.label}>
                <a
                  href={link.href}
                  {...link.track}
                  data-track-location="footer"
                  {...(link.external ? { target: "_blank", rel: "noreferrer" } : {})}
                  className="text-muted-foreground transition-colors hover:text-foreground"
                >
                  {link.label}
                </a>
              </li>
            ))}
          </ul>
        </nav>
      </Container>
      <Container className="flex flex-col gap-2 border-t border-border py-6 text-sm text-muted-foreground sm:flex-row sm:justify-between">
        <p>
          © 2026 Kopi ·{" "}
          <a
            href={siteConfig.links.license}
            data-track="license_click"
            data-track-location="footer"
            target="_blank"
            rel="noreferrer"
            className="transition-colors hover:text-foreground"
          >
            MIT licensed
          </a>
        </p>
        <p>
          Made by{" "}
          {/* No noreferrer, so the portfolio's analytics see visits from Kopi. */}
          <a
            href={siteConfig.links.author}
            data-track="author_click"
            data-track-location="footer"
            target="_blank"
            rel="author noopener"
            className="font-medium text-foreground transition-colors hover:text-accent-text"
          >
            Devanshu Verma
          </a>
        </p>
      </Container>
    </footer>
  );
}
