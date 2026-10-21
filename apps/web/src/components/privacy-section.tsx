import { EyeOff, FileCode2, HardDrive, KeyRound } from "lucide-react";

import captureDark from "@/assets/screens/capture-dark.png";
import captureLight from "@/assets/screens/capture-light.png";
import { AppShot, WindowFrame } from "@/components/ui/app-shot";

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
          <WindowFrame title="Kopi Settings" className="mx-auto max-w-lg">
            <AppShot
              light={captureLight}
              dark={captureDark}
              alt="Kopi's Capture settings: the kinds of clips to keep, and excluded apps, with 1Password, Keychain Access and Passwords built in."
              sizes="(min-width: 1024px) 512px, calc(100vw - 2rem)"
            />
          </WindowFrame>
        </Reveal>
      </div>
    </Section>
  );
}
