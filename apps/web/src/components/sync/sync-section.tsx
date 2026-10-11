import { Check } from "lucide-react";

import { SyncAnimation } from "@/components/sync/sync-animation";
import { Reveal } from "@/components/ui/reveal";
import { Section, SectionHeader } from "@/components/ui/section";

const points = [
  "Copy on your Mac, paste on your Windows PC a moment later",
  "Clips are encrypted on your device before they leave it",
  "Choose what syncs: everything, pinboards only, or nothing at all",
];

export function SyncSection() {
  return (
    <Section id="sync" labelledBy="sync-title" className="bg-surface-muted/50">
      <SectionHeader
        id="sync-title"
        title="Copy on your Mac. Paste on your PC."
        description="Plenty of us work across both. Kopi keeps one clipboard history across your computers, whichever operating system they run."
      />
      <Reveal className="mt-14">
        <SyncAnimation />
      </Reveal>
      <Reveal className="mt-12">
        <ul className="grid gap-4 sm:grid-cols-3">
          {points.map((point) => (
            <li key={point} className="flex gap-3 text-sm leading-relaxed text-muted-foreground">
              <Check className="mt-0.5 size-4 shrink-0 text-accent-text" aria-hidden="true" />
              {point}
            </li>
          ))}
        </ul>
      </Reveal>
    </Section>
  );
}
