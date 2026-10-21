import { Bell, FlaskConical, MessageSquare } from "lucide-react";

import { Reveal } from "@/components/ui/reveal";
import { Section } from "@/components/ui/section";
import { WaitlistForm } from "@/components/waitlist/waitlist-form";

const perks = [
  { icon: FlaskConical, title: "Early access", text: "A place in the beta, on Mac or Windows" },
  { icon: Bell, title: "Only real news", text: "A short note when there's real progress, nothing more" },
  { icon: MessageSquare, title: "A say", text: "Tell us what to build first" },
];

export function WaitlistSection() {
  return (
    <Section id="waitlist" labelledBy="waitlist-title">
      <div className="relative overflow-hidden rounded-4xl border border-border bg-[linear-gradient(160deg,var(--accent-soft),var(--surface-muted)_55%,var(--background))] p-5 sm:p-10 lg:p-14">
        <div
          aria-hidden="true"
          className="pointer-events-none absolute -top-1/3 -left-1/4 size-144 rounded-full bg-[radial-gradient(closest-side,color-mix(in_oklab,var(--accent)_28%,transparent),transparent)] blur-2xl"
        />
        <div className="relative grid gap-10 lg:grid-cols-[0.9fr_1.1fr] lg:items-center lg:gap-14">
          <Reveal className="px-1 sm:px-0">
            <p className="text-sm font-semibold text-accent-text">Beta waitlist</p>
            <h2 id="waitlist-title" className="mt-4 text-3xl leading-[1.08] font-semibold tracking-[-0.03em] sm:text-5xl">
              Be the first to try what&apos;s next
            </h2>
            <p className="mt-5 text-lg leading-relaxed text-muted-foreground">
              The alpha is out for Mac and Windows. Leave your email to hear about the beta, and tell us whether
              you&apos;d like to use Kopi, test it or help build it.
            </p>
            <ul className="mt-8 space-y-4">
              {perks.map(({ icon: Icon, title, text }) => (
                <li key={title} className="flex items-start gap-3.5">
                  <span className="inline-flex size-9 shrink-0 items-center justify-center rounded-xl border border-border bg-surface shadow-soft">
                    <Icon className="size-4 text-accent-text" aria-hidden="true" />
                  </span>
                  <span className="text-sm leading-relaxed">
                    <span className="block font-semibold">{title}</span>
                    <span className="text-muted-foreground">{text}</span>
                  </span>
                </li>
              ))}
            </ul>
          </Reveal>
          <Reveal delay={0.1} className="relative">
            <WaitlistForm />
          </Reveal>
        </div>
      </div>
    </Section>
  );
}
