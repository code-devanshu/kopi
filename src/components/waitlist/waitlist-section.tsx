import { Bell, FlaskConical, MessageSquare } from "lucide-react";

import { Reveal } from "@/components/ui/reveal";
import { Section, SectionHeader } from "@/components/ui/section";
import { WaitlistForm } from "@/components/waitlist/waitlist-form";

const perks = [
  { icon: FlaskConical, text: "Early access to alpha builds on your platform" },
  { icon: Bell, text: "A short note when there's real progress, nothing more" },
  { icon: MessageSquare, text: "A say in what gets built first" },
];

export function WaitlistSection() {
  return (
    <Section id="waitlist" labelledBy="waitlist-title" className="bg-surface-muted/50">
      <div className="grid gap-12 lg:grid-cols-[0.9fr_1.1fr] lg:items-center">
        <div>
          <SectionHeader
            id="waitlist-title"
            title="Be the first to try Kopi"
            description="Kopi isn't out yet. Leave your email and tell us whether you'd like to use it, test it or help build it."
          />
          <Reveal delay={0.1}>
            <ul className="mt-8 space-y-4">
              {perks.map(({ icon: Icon, text }) => (
                <li key={text} className="flex items-center gap-3 text-sm">
                  <Icon className="size-4 shrink-0 text-accent-text" aria-hidden="true" />
                  {text}
                </li>
              ))}
            </ul>
          </Reveal>
        </div>
        <Reveal delay={0.1} className="relative">
          <WaitlistForm />
        </Reveal>
      </div>
    </Section>
  );
}
