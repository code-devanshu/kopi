import { Bug, Code2, Palette } from "lucide-react";

import { GitHubIcon } from "@/components/brand/brand-icons";
import { buttonStyles } from "@/components/ui/button-styles";
import { Reveal } from "@/components/ui/reveal";
import { Section, SectionHeader } from "@/components/ui/section";
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

export function OpenSource() {
  return (
    <Section id="open-source" labelledBy="open-source-title">
      <SectionHeader
        id="open-source-title"
        title="Built in the open. Come build it with us."
        description="The code, roadmap and discussions are public, and there's room for every kind of contributor."
      />
      <ul className="mt-12 grid gap-8 md:grid-cols-3 md:gap-10">
        {roles.map(({ icon: Icon, title, description }, i) => (
          <Reveal as="li" key={title} delay={i * 0.06} className="border-t border-border pt-6">
            <h3 className="flex items-center gap-2.5 text-lg font-semibold">
              <Icon className="size-5 text-accent-text" aria-hidden="true" />
              {title}
            </h3>
            <p className="mt-2 text-sm leading-relaxed text-muted-foreground">{description}</p>
          </Reveal>
        ))}
      </ul>
      <Reveal className="mt-10 flex flex-col gap-3 sm:flex-row">
        <a href={siteConfig.links.github} data-track="github_click" data-track-location="open_source" target="_blank" rel="noreferrer" className={buttonStyles({ size: "lg" })}>
          <GitHubIcon className="size-4" />
          View on GitHub
        </a>
        <a href="#waitlist" data-track="waitlist_cta_click" data-track-location="open_source" className={buttonStyles({ variant: "secondary", size: "lg" })}>
          Sign up to help
        </a>
      </Reveal>
    </Section>
  );
}
