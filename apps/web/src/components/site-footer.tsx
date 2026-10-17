import { GitHubIcon } from "@/components/brand/brand-icons";
import { Logo } from "@/components/brand/logo";
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
        <div className="max-w-xs">
          <Logo />
          <p className="mt-4 text-sm leading-relaxed text-muted-foreground">
            An open-source clipboard manager. The alpha is out for macOS and Windows.
          </p>
          <div className="mt-5 flex gap-2">
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
