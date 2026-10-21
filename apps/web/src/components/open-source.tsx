import { ArrowRight, Bug, Code2, Palette } from "lucide-react";

import { GitHubIcon } from "@/components/brand/brand-icons";
import { buttonStyles } from "@/components/ui/button-styles";
import { Reveal } from "@/components/ui/reveal";
import { Section } from "@/components/ui/section";
import { siteConfig } from "@/lib/site";

const roles = [
  {
    icon: Code2,
    title: "Developers",
    description: "Help build the core app, the platform integrations and the encrypted sync layer.",
  },
  {
    icon: Palette,
    title: "Designers",
    description: "Shape the interface, icons and onboarding so Kopi feels at home on both platforms.",
  },
  {
    icon: Bug,
    title: "Testers",
    description: "Try early builds on your own setup and tell us what breaks, confuses or delights you.",
  },
];

const stack = ["MIT license", "Rust", "Tauri 2", "React"];

// The same steps as the README's "Build from source".
const commands = [
  { prompt: true, text: `git clone ${siteConfig.links.github}.git` },
  { prompt: true, text: "cd kopi && pnpm install" },
  { prompt: true, text: "pnpm dev:desktop" },
  { prompt: false, text: "# Kopi opens, built from your copy of the code" },
];

export function OpenSource() {
  return (
    <Section id="open-source" labelledBy="open-source-title">
      <Reveal className="overflow-hidden rounded-4xl border border-border bg-surface shadow-soft">
        <div className="grid items-center gap-10 p-6 sm:p-10 lg:grid-cols-[1fr_1.05fr] lg:gap-14 lg:p-14">
          <div>
            <p className="text-sm font-semibold text-accent-text">Open source</p>
            <h2 id="open-source-title" className="mt-4 text-3xl leading-[1.08] font-semibold tracking-[-0.03em] sm:text-5xl">
              Built in the open. Come build it with us.
            </h2>
            <p className="mt-5 text-lg leading-relaxed text-muted-foreground">
              The code, roadmap and discussions are public, and there&apos;s room for every kind of contributor.
            </p>
            <ul className="mt-6 flex flex-wrap gap-2" aria-label="License and stack">
              {stack.map((item) => (
                <li key={item} className="rounded-full border border-border bg-background px-3 py-1 text-xs font-medium text-muted-foreground">
                  {item}
                </li>
              ))}
            </ul>
            <div className="mt-8 flex flex-col gap-3 sm:flex-row">
              <a href={siteConfig.links.github} data-track="github_click" data-track-location="open_source" target="_blank" rel="noreferrer" className={buttonStyles({ size: "lg" })}>
                <GitHubIcon className="size-4" />
                View on GitHub
              </a>
              <a href="#waitlist" data-track="waitlist_cta_click" data-track-location="open_source" className={buttonStyles({ variant: "secondary", size: "lg" })}>
                Sign up to help
                <ArrowRight className="size-4" aria-hidden="true" />
              </a>
            </div>
          </div>

          {/* A terminal, dark in both themes. */}
          <div data-theme="dark" className="min-w-0 overflow-hidden rounded-2xl border border-border bg-background text-foreground shadow-[0_30px_60px_-24px_rgb(0_0_0/0.55)]">
            <div className="relative flex h-10 items-center border-b border-border bg-surface px-4" aria-hidden="true">
              <span className="flex gap-2">
                <span className="size-3 rounded-full bg-[#ff5f57]" />
                <span className="size-3 rounded-full bg-[#febc2e]" />
                <span className="size-3 rounded-full bg-[#28c840]" />
              </span>
              <span className="absolute inset-x-0 text-center text-xs font-medium text-muted-foreground">Terminal</span>
            </div>
            <pre className="p-5 font-mono text-xs leading-7 whitespace-pre-wrap [overflow-wrap:anywhere] sm:p-6 sm:text-[13px] sm:leading-8">
              <code>
                {commands.map(({ prompt, text }) => (
                  <span key={text} className="block">
                    {prompt ? (
                      <>
                        <span className="text-accent-text select-none">$ </span>
                        {text}
                      </>
                    ) : (
                      <span className="text-muted-foreground">{text}</span>
                    )}
                  </span>
                ))}
              </code>
            </pre>
          </div>
        </div>

        <ul className="grid border-t border-border bg-surface-muted/40 md:grid-cols-3">
          {roles.map(({ icon: Icon, title, description }) => (
            <li key={title} className="border-border p-6 not-last:border-b sm:p-8 md:not-last:border-r md:not-last:border-b-0">
              <h3 className="flex items-center gap-2.5 font-semibold">
                <Icon className="size-5 text-accent-text" aria-hidden="true" />
                {title}
              </h3>
              <p className="mt-2 text-sm leading-relaxed text-muted-foreground">{description}</p>
            </li>
          ))}
        </ul>
      </Reveal>
    </Section>
  );
}
