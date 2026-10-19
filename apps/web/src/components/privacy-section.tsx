import { EyeOff, FileCode2, HardDrive, KeyRound, ShieldCheck } from "lucide-react";

import { Reveal } from "@/components/ui/reveal";
import { Section, SectionHeader } from "@/components/ui/section";

const principles = [
  {
    icon: HardDrive,
    title: "Local-first storage",
    description: "Your history lives in a database on your own computer. Kopi works fully offline and never needs an account to be useful.",
  },
  {
    icon: KeyRound,
    title: "End-to-end encrypted sync",
    description: "Sync is optional and off until you turn it on. Clips go straight between your computers on your network, encrypted with keys that never leave them, and never through a server.",
  },
  {
    icon: EyeOff,
    title: "Skips sensitive content",
    description: "Copies from password managers and content marked as concealed are ignored. You can add your own apps to the ignore list too.",
  },
  {
    icon: FileCode2,
    title: "Open source, open to audit",
    description: "Every line of Kopi is public. Anyone can read how your data is handled, and check that the claims on this page hold up.",
  },
];

function SkippedClipMock() {
  return (
    <div aria-hidden="true" className="rounded-2xl border border-border bg-surface p-5 shadow-lifted">
      <div className="flex items-center gap-2 text-sm font-medium">
        <ShieldCheck className="size-4 text-success" />
        Privacy log
      </div>
      <ul className="mt-4 space-y-2.5 text-xs">
        <li className="flex items-center justify-between gap-3 rounded-lg bg-surface-muted px-3 py-2.5">
          <span className="flex items-center gap-2">
            <span className="font-mono tracking-widest">••••••••••</span>
            <span className="text-muted-foreground">Password manager</span>
          </span>
          <span className="rounded-full border border-border px-2 py-0.5 text-muted-foreground">Skipped</span>
        </li>
        <li className="flex items-center justify-between gap-3 rounded-lg bg-surface-muted px-3 py-2.5">
          <span className="flex items-center gap-2">
            <span className="font-mono tracking-widest">•••• 4821</span>
            <span className="text-muted-foreground">Marked concealed</span>
          </span>
          <span className="rounded-full border border-border px-2 py-0.5 text-muted-foreground">Skipped</span>
        </li>
        <li className="flex items-center justify-between gap-3 rounded-lg bg-surface-muted px-3 py-2.5">
          <span className="truncate">Quarterly plan (final).pdf</span>
          <span className="rounded-full bg-accent-soft px-2 py-0.5 font-medium text-accent-text">Saved locally</span>
        </li>
      </ul>
      <p className="mt-4 text-xs text-muted-foreground">Stored on this computer · Never sent anywhere</p>
    </div>
  );
}

export function PrivacySection() {
  return (
    <Section id="privacy" labelledBy="privacy-title">
      <div className="grid items-center gap-12 lg:grid-cols-[1fr_0.85fr]">
        <div>
          <SectionHeader
            id="privacy-title"
            title="Your clipboard is personal. Kopi treats it that way."
            description="A clipboard sees passwords, addresses and half-written messages. Kopi is designed so that none of it is anyone else's business."
          />
          <ul className="mt-10 grid gap-6 sm:grid-cols-2">
            {principles.map(({ icon: Icon, title, description }, i) => (
              <Reveal as="li" key={title} delay={i * 0.06}>
                <Icon className="size-5 text-accent-text" aria-hidden="true" />
                <h3 className="mt-3 font-semibold">{title}</h3>
                <p className="mt-2 text-sm leading-relaxed text-muted-foreground">{description}</p>
              </Reveal>
            ))}
          </ul>
        </div>
        <Reveal delay={0.1}>
          <SkippedClipMock />
        </Reveal>
      </div>
    </Section>
  );
}
