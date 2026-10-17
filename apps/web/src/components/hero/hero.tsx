import { HeroCardStack } from "@/components/hero/hero-card-stack";
import { HeroDownloads } from "@/components/hero/hero-downloads";
import { InstallCommand } from "@/components/hero/install-command";
import { PlatformNote } from "@/components/hero/platform-note";
import { Reveal } from "@/components/ui/reveal";
import { Container } from "@/components/ui/section";

export function Hero() {
  return (
    <section id="top" aria-labelledby="hero-title" className="relative overflow-hidden">
      {/* Gradient + grain live only in the hero. */}
      <div
        aria-hidden="true"
        className="pointer-events-none absolute inset-0 bg-[radial-gradient(60%_50%_at_75%_30%,var(--accent-soft),transparent_70%),radial-gradient(50%_40%_at_10%_0%,var(--surface-muted),transparent_70%)]"
      />
      <div aria-hidden="true" className="hero-grain pointer-events-none absolute inset-0" />

      {/* Both columns sit at the top, so opening the install command grows the hero downwards instead of moving the cards. */}
      <Container className="relative grid items-start gap-8 pt-14 pb-20 sm:pt-20 lg:grid-cols-[1.05fr_1fr] lg:gap-12 lg:pt-24 lg:pb-28">
        {/* min-w-0 lets the column shrink; otherwise the one-line install command widens it past the screen. */}
        <div className="min-w-0">
          <Reveal>
            <p className="inline-flex rounded-full border border-border bg-surface/70 px-3 py-1 text-xs font-medium text-muted-foreground">
              Open source · Free · Alpha for Mac and Windows
            </p>
          </Reveal>

          <Reveal delay={0.05}>
            <h1 id="hero-title" className="mt-6 text-4xl leading-[1.05] font-semibold sm:text-5xl lg:text-6xl">
              Everything you copy, on every computer.
            </h1>
          </Reveal>

          <Reveal delay={0.1}>
            <p className="mt-6 max-w-xl text-lg leading-relaxed text-muted-foreground">
              Kopi keeps everything you copy, searchable in a keystroke and synced between Mac and
              Windows. Open source and private.
            </p>
          </Reveal>

          <Reveal delay={0.15} className="mt-8">
            <HeroDownloads />
          </Reveal>

          <Reveal delay={0.2} className="mt-5 space-y-3">
            <PlatformNote />
            <InstallCommand />
          </Reveal>
        </div>

        <HeroCardStack />
      </Container>
    </section>
  );
}
