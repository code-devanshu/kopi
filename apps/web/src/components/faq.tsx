"use client";

import { AnimatePresence, motion } from "motion/react";
import { ChevronDown } from "lucide-react";
import { useState, type ReactNode } from "react";

import { Reveal } from "@/components/ui/reveal";
import { Section, SectionHeader } from "@/components/ui/section";
import { trackEvent } from "@/lib/analytics";
import { siteConfig } from "@/lib/site";

const faqs: Array<{ question: string; answer: ReactNode }> = [
  {
    question: "Is Kopi free?",
    answer:
      "Yes. Kopi is free and open source. You'll be able to download it, read the code, and build it yourself if you prefer.",
  },
  {
    question: "When does it launch?",
    answer:
      "There's no date yet, and we'd rather ship something solid than rush it. The roadmap above shows where things stand. Join the waitlist and we'll let you know when alpha builds are ready.",
  },
  {
    question: "Which platforms will it support?",
    answer:
      "macOS and Windows are the focus for 1.0, with sync between them. If you'd like to see another platform, start a discussion on GitHub.",
  },
  {
    question: "Is my data private?",
    answer:
      "Your history is stored locally on your computer. Sync is optional, and when it's on, clips are end-to-end encrypted before they leave your device. Kopi skips copies from password managers and content marked as sensitive, and because the code is open, anyone can verify how it all works.",
  },
  {
    question: "How can I contribute?",
    answer: (
      <>
        Star and watch the{" "}
        <a href={siteConfig.links.github} data-track="github_click" data-track-location="faq" target="_blank" rel="noreferrer" className="font-medium text-accent-text underline underline-offset-4">
          GitHub repository
        </a>{" "}
        to follow along, open issues with ideas, or{" "}
        <a href="#waitlist" data-track="waitlist_cta_click" data-track-location="faq" className="font-medium text-accent-text underline underline-offset-4">
          use the form
        </a>{" "}
        to tell us whether you&apos;d like to code, design or test. Every kind of help is welcome.
      </>
    ),
  },
];

function FaqItem({ question, answer, index }: { question: string; answer: ReactNode; index: number }) {
  const [open, setOpen] = useState(false);
  const buttonId = `faq-question-${index}`;
  const panelId = `faq-answer-${index}`;

  return (
    <li className="border-b border-border last:border-b-0">
      <h3>
        <button
          id={buttonId}
          type="button"
          aria-expanded={open}
          aria-controls={panelId}
          onClick={() => {
            if (!open) trackEvent("faq_open", { question });
            setOpen(!open);
          }}
          className="flex w-full items-center justify-between gap-4 py-5 text-left text-base font-medium sm:text-lg"
        >
          {question}
          <motion.span animate={{ rotate: open ? 180 : 0 }} transition={{ duration: 0.2 }} className="shrink-0">
            <ChevronDown className="size-5 text-muted-foreground" aria-hidden="true" />
          </motion.span>
        </button>
      </h3>
      <AnimatePresence initial={false}>
        {open ? (
          <motion.div
            id={panelId}
            role="region"
            aria-labelledby={buttonId}
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: "auto", opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            transition={{ duration: 0.25, ease: [0.22, 1, 0.36, 1] }}
            className="overflow-hidden"
          >
            <p className="pb-5 leading-relaxed text-muted-foreground">{answer}</p>
          </motion.div>
        ) : null}
      </AnimatePresence>
    </li>
  );
}

export function Faq() {
  return (
    <Section id="faq" labelledBy="faq-title">
      <div className="grid gap-10 lg:grid-cols-[1fr_2fr] lg:gap-16">
        <SectionHeader id="faq-title" title="Questions, answered" />
        <Reveal>
          <ul className="-mt-5">
            {faqs.map((faq, i) => (
              <FaqItem key={faq.question} index={i} {...faq} />
            ))}
          </ul>
        </Reveal>
      </div>
    </Section>
  );
}
