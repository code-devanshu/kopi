import { HardDrive, KeyRound, Laptop, Scale } from "lucide-react";

import { Container } from "@/components/ui/section";

// Claims only the code backs up, as the privacy section says.
const items = [
  { icon: Scale, text: "Open source, MIT licensed" },
  { icon: HardDrive, text: "Stays on your computer" },
  { icon: KeyRound, text: "No account, no cloud" },
  { icon: Laptop, text: "macOS and Windows" },
];

export function TrustStrip() {
  return (
    <div className="border-y border-border bg-surface/60">
      <Container>
        <ul className="grid grid-cols-2 gap-x-6 gap-y-4 py-6 sm:py-7 lg:grid-cols-4">
          {items.map(({ icon: Icon, text }) => (
            <li key={text} className="flex items-center justify-center gap-2.5 text-sm font-medium text-muted-foreground">
              <Icon className="size-4 shrink-0 text-accent-text" aria-hidden="true" />
              {text}
            </li>
          ))}
        </ul>
      </Container>
    </div>
  );
}
