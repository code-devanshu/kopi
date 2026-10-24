import { ProductHuntLaunch } from "@/components/brand/product-hunt-badge";
import { HeroDownloads } from "@/components/hero/hero-downloads";
import { HeroShot } from "@/components/hero/hero-shot";
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
        className="pointer-events-none absolute inset-0 bg-[radial-gradient(55%_45%_at_50%_0%,var(--accent-soft),transparent_75%),radial-gradient(40%_35%_at_85%_60%,var(--surface-muted),transparent_70%)]"
      />
      <div aria-hidden="true" className="hero-grain pointer-events-none absolute inset-0" />

      <Container className="relative pt-16 text-center sm:pt-24 lg:pt-28">
        <Reveal>
          <p className="inline-flex rounded-full border border-border bg-surface/70 px-3 py-1 text-xs font-medium text-muted-foreground">
            Open source · Free · Alpha for Mac and Windows
          </p>
        </Reveal>

        <Reveal delay={0.05}>
          <h1
            id="hero-title"
            className="mx-auto mt-7 max-w-4xl text-5xl leading-[1.02] font-semibold tracking-[-0.035em] sm:text-6xl lg:text-7xl"
          >
            Your clipboard, with a memory.
          </h1>
        </Reveal>

        <Reveal delay={0.1}>
          <p className="mx-auto mt-6 max-w-2xl text-lg leading-relaxed text-muted-foreground sm:text-xl">
            Everything you copy on your Mac or PC, a keystroke away, even the words in your
            screenshots. Private, open source, and synced between your computers over your own network.
          </p>
        </Reveal>

        <Reveal delay={0.15} className="mt-9">
          <HeroDownloads className="justify-center" />
          <ProductHuntLaunch location="hero" className="mt-6" />
        </Reveal>

        <Reveal delay={0.2} className="mt-5 space-y-3">
          <PlatformNote />
          <InstallCommand />
        </Reveal>
      </Container>

      <Container className="relative mt-14 pb-20 sm:mt-20 sm:pb-28">
        <HeroShot />
      </Container>
    </section>
  );
}
