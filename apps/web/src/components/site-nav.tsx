"use client";

import { AnimatePresence, motion } from "motion/react";
import { Menu, X } from "lucide-react";
import { useEffect, useState } from "react";

import { GitHubIcon } from "@/components/brand/brand-icons";
import { Logo } from "@/components/brand/logo";
import { ThemeToggle } from "@/components/theme/theme-toggle";
import { buttonStyles } from "@/components/ui/button-styles";
import { Container } from "@/components/ui/section";
import { navLinks, siteConfig } from "@/lib/site";

export function SiteNav() {
  const [open, setOpen] = useState(false);

  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && setOpen(false);
    const onResize = () => window.innerWidth >= 768 && setOpen(false);
    window.addEventListener("keydown", onKey);
    window.addEventListener("resize", onResize);
    return () => {
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("resize", onResize);
    };
  }, [open]);

  return (
    <header className="sticky top-0 z-50 border-b border-border/70 bg-background/80 backdrop-blur-md supports-[backdrop-filter]:bg-background/70">
      <Container className="flex h-16 items-center justify-between gap-4">
        <a href="#top" className="rounded-lg" aria-label="Kopi, back to top">
          <Logo />
        </a>

        <nav aria-label="Main" className="hidden md:block">
          <ul className="flex items-center gap-1">
            {navLinks.map((link) => (
              <li key={link.href}>
                <a
                  href={link.href}
                  data-track="nav_click"
                  data-track-section={link.href.slice(1)}
                  data-track-location="nav"
                  className="rounded-full px-3 py-2 text-sm text-muted-foreground transition-colors hover:text-foreground"
                >
                  {link.label}
                </a>
              </li>
            ))}
          </ul>
        </nav>

        <div className="flex items-center gap-1.5">
          <ThemeToggle />
          <a
            href={siteConfig.links.github}
            data-track="github_click" data-track-location="nav"
            target="_blank"
            rel="noreferrer"
            className={buttonStyles({ variant: "ghost", size: "sm", className: "hidden sm:inline-flex" })}
          >
            <GitHubIcon className="size-4" />
            GitHub
          </a>
          <a
            href={siteConfig.links.releases}
            data-track="download_click" data-track-platform="macos" data-track-location="nav"
            target="_blank"
            rel="noreferrer"
            className={buttonStyles({ size: "sm", className: "hidden sm:inline-flex" })}
          >
            Download
          </a>
          <button
            type="button"
            className="inline-flex size-9 items-center justify-center rounded-full hover:bg-surface-muted md:hidden"
            aria-expanded={open}
            aria-controls="mobile-menu"
            aria-label={open ? "Close menu" : "Open menu"}
            onClick={() => setOpen((v) => !v)}
          >
            {open ? <X className="size-5" aria-hidden="true" /> : <Menu className="size-5" aria-hidden="true" />}
          </button>
        </div>
      </Container>

      <AnimatePresence>
        {open ? (
          <motion.nav
            id="mobile-menu"
            aria-label="Mobile"
            initial={{ opacity: 0, height: 0 }}
            animate={{ opacity: 1, height: "auto" }}
            exit={{ opacity: 0, height: 0 }}
            transition={{ duration: 0.25, ease: [0.22, 1, 0.36, 1] }}
            className="overflow-hidden border-t border-border md:hidden"
          >
            <Container className="flex flex-col gap-1 py-4">
              {navLinks.map((link) => (
                <a
                  key={link.href}
                  href={link.href}
                  data-track="nav_click"
                  data-track-section={link.href.slice(1)}
                  data-track-location="nav_mobile"
                  onClick={() => setOpen(false)}
                  className="rounded-xl px-3 py-3 text-base text-foreground hover:bg-surface-muted"
                >
                  {link.label}
                </a>
              ))}
              <div className="mt-3 grid grid-cols-2 gap-2">
                <a
                  href={siteConfig.links.github}
                  data-track="github_click" data-track-location="nav_mobile"
                  target="_blank"
                  rel="noreferrer"
                  className={buttonStyles({ variant: "secondary" })}
                >
                  <GitHubIcon className="size-4" />
                  GitHub
                </a>
                <a
                  href={siteConfig.links.releases}
                  data-track="download_click" data-track-platform="macos" data-track-location="nav_mobile"
                  target="_blank"
                  rel="noreferrer"
                  className={buttonStyles()}
                >
                  Download
                </a>
              </div>
            </Container>
          </motion.nav>
        ) : null}
      </AnimatePresence>
    </header>
  );
}
