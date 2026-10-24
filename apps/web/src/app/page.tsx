import { Faq } from "@/components/faq";
import { Features } from "@/components/features/features";
import { FinalCta } from "@/components/final-cta";
import { Hero } from "@/components/hero/hero";
import { OpenSource } from "@/components/open-source";
import { PrivacySection } from "@/components/privacy-section";
import { Roadmap } from "@/components/roadmap";
import { SiteFooter } from "@/components/site-footer";
import { SiteNav } from "@/components/site-nav";
import { SyncSection } from "@/components/sync/sync-section";
import { TrustStrip } from "@/components/trust-strip";
import { UseCases } from "@/components/use-cases/use-cases";
import { WaitlistSection } from "@/components/waitlist/waitlist-section";

export default function Home() {
  return (
    <>
      <SiteNav />
      <main id="main">
        <Hero />
        <TrustStrip />
        <Features />
        <SyncSection />
        <PrivacySection />
        <UseCases />
        <OpenSource />
        <Roadmap />
        <WaitlistSection />
        <Faq />
        <FinalCta />
      </main>
      <SiteFooter />
    </>
  );
}
