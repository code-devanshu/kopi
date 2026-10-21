import Image from "next/image";
import { Check } from "lucide-react";

import syncDark from "@/assets/screens/sync-dark.png";
import { WindowFrame } from "@/components/ui/app-shot";
import { Reveal } from "@/components/ui/reveal";
import { Container } from "@/components/ui/section";

const points = [
  "Copy on your Mac, paste on your Windows PC a moment later",
  "Encrypted end to end, straight between your computers on your network, never through a server",
  "Off until you turn it on, and you choose which kinds of clips sync",
];

/** The page's one dark band, in both themes, to break up the cream. */
export function SyncSection() {
  return (
    <section
      id="sync"
      aria-labelledby="sync-title"
      data-theme="dark"
      className="relative overflow-hidden bg-surface py-24 text-foreground sm:py-32"
    >
      <div
        aria-hidden="true"
        className="pointer-events-none absolute inset-0 bg-[radial-gradient(45%_60%_at_80%_50%,color-mix(in_oklab,var(--accent)_22%,transparent),transparent_70%)]"
      />
      <Container className="relative grid items-center gap-14 lg:grid-cols-[0.9fr_1.1fr] lg:gap-16">
        <Reveal>
          <p className="text-sm font-semibold text-accent-text">Encrypted sync</p>
          <h2 id="sync-title" className="mt-4 text-4xl leading-[1.05] font-semibold tracking-[-0.03em] sm:text-5xl">
            Copy on your Mac.
            <br />
            Paste on your PC.
          </h2>
          <p className="mt-6 text-lg leading-relaxed text-muted-foreground">
            Plenty of us work across both. Kopi keeps one clipboard history, with your pinboards and
            snippets, across your computers on the same network, whichever operating system they run.
          </p>
          <ul className="mt-8 space-y-3.5">
            {points.map((point) => (
              <li key={point} className="flex gap-3 leading-relaxed">
                <Check className="mt-1 size-4 shrink-0 text-accent-text" aria-hidden="true" />
                {point}
              </li>
            ))}
          </ul>
        </Reveal>

        <Reveal delay={0.1}>
          <WindowFrame title="Kopi Settings" className="mx-auto max-w-xl shadow-[0_40px_80px_-24px_rgb(0_0_0/0.7)]">
            <Image
              src={syncDark}
              alt="Kopi's Sync settings, with sync on and two paired computers: a Windows PC, connected, and a Mac mini."
              sizes="(min-width: 1024px) 576px, calc(100vw - 2rem)"
              quality={90}
              className="h-auto w-full"
            />
          </WindowFrame>
        </Reveal>
      </Container>
    </section>
  );
}
