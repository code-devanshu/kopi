import { LogoMark } from "@/components/brand/logo";
import { HeroDownloads } from "@/components/hero/hero-downloads";
import { Reveal } from "@/components/ui/reveal";
import { Container } from "@/components/ui/section";

export function FinalCta() {
  return (
    <section id="get-kopi" aria-labelledby="final-cta-title" className="relative overflow-hidden py-24 sm:py-32">
      <div
        aria-hidden="true"
        className="pointer-events-none absolute inset-0 bg-[radial-gradient(40%_45%_at_50%_55%,var(--accent-soft),transparent_80%)]"
      />
      <Container className="relative text-center">
        <Reveal>
          <LogoMark className="mx-auto size-16 drop-shadow-[0_12px_24px_rgb(var(--shadow-color)/0.25)]" />
          <h2
            id="final-cta-title"
            className="mx-auto mt-8 max-w-3xl text-4xl leading-[1.05] font-semibold tracking-[-0.035em] sm:text-6xl"
          >
            Copy once. Find it whenever.
          </h2>
          <p className="mx-auto mt-5 max-w-xl text-lg text-muted-foreground">
            Free and open source, for macOS 14 or later and Windows 10 and 11. Kopi updates itself after
            the first install.
          </p>
        </Reveal>
        <Reveal delay={0.1} className="mt-9">
          <HeroDownloads location="final_cta" className="justify-center" />
        </Reveal>
      </Container>
    </section>
  );
}
