"use client";

import { AnimatePresence, motion } from "motion/react";
import { Plus } from "lucide-react";
import { useState, type ReactNode } from "react";

import { Reveal } from "@/components/ui/reveal";
import { Container } from "@/components/ui/section";
import { trackEvent } from "@/lib/analytics";
import { cn } from "@/lib/cn";
import { siteConfig } from "@/lib/site";

const link = "font-medium text-accent-text underline underline-offset-4";

const faqs: Array<{ question: string; answer: ReactNode }> = [
  {
    question: "Is Kopi free?",
    answer:
      "Yes. Kopi is free and open source. Download it, read the code, or build it yourself if you prefer.",
  },
  {
    question: "Can I use it now?",
    answer: (
      <>
        Yes, on a Mac or a Windows PC.{" "}
        <a href="#get-kopi" data-track="download_cta_click" data-track-location="faq" className={link}>
          Download the alpha
        </a>
        , and Kopi updates itself after that. It&apos;s early, and Windows is the newest, so expect rough edges and{" "}
        <a href={`${siteConfig.links.github}/issues/new/choose`} data-track="github_click" data-track-location="faq_issues" target="_blank" rel="noreferrer" className={link}>
          tell us what you find
        </a>
        .
      </>
    ),
  },
  {
    question: "Why does my computer warn me when I first open Kopi?",
    answer: (
      <>
        Apple&apos;s notarization and Windows code signing both need paid developer accounts, and Kopi doesn&apos;t
        have them yet. On a Mac, the one-line install from Terminal avoids the warning. With the download, macOS
        blocks Kopi once, and you allow it in System Settings. On Windows, choose More info, then Run anyway. The{" "}
        <a href={siteConfig.links.install} data-track="install_guide_click" data-track-location="faq" target="_blank" rel="noreferrer" className={link}>
          install guide
        </a>{" "}
        has the steps. You do this once, and updates install without it.
      </>
    ),
  },
  {
    question: "What does Kopi need permission for?",
    answer:
      "On a Mac, Accessibility lets Kopi paste into the app you're using. Without it, Kopi copies the clip and you press ⌘V yourself. Newer versions of macOS also ask before an app reads what you copy, so set Kopi to Always Allow. The setup window walks you through both.",
  },
  {
    question: "Is my data private?",
    answer:
      "Your history is stored locally on your computer. Sync is optional and off until you turn it on. When it's on, clips go straight between your computers on your local network, end-to-end encrypted, and never through a server. The text in images you copy is read on your computer too, never sent anywhere, and you can turn that off. Kopi skips copies from password managers and content marked as sensitive, and because the code is open, anyone can verify how it all works.",
  },
  {
    question: "Which platforms will it support?",
    answer:
      "macOS and Windows, both in alpha now, with optional encrypted sync between them over your local network. They're the focus for 1.0. If you'd like to see another platform, start a discussion on GitHub.",
  },
  {
    question: "How can I contribute?",
    answer: (
      <>
        Star and watch the{" "}
        <a href={siteConfig.links.github} data-track="github_click" data-track-location="faq" target="_blank" rel="noreferrer" className={link}>
          GitHub repository
        </a>{" "}
        to follow along, open issues with ideas, or{" "}
        <a href="#waitlist" data-track="waitlist_cta_click" data-track-location="faq" className={link}>
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
    <li
      className={cn(
        "rounded-2xl border bg-surface transition-[border-color,box-shadow] duration-200",
        open ? "border-[color-mix(in_oklab,var(--border)_50%,var(--accent))] shadow-soft" : "border-border",
      )}
    >
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
          className="flex w-full items-center justify-between gap-5 rounded-2xl px-5 py-5 text-left text-base font-medium sm:px-6 sm:text-lg"
        >
          {question}
          <span
            aria-hidden="true"
            className={cn(
              "inline-flex size-8 shrink-0 items-center justify-center rounded-full border transition-colors",
              open ? "border-accent bg-accent text-accent-foreground" : "border-border text-muted-foreground",
            )}
          >
            <motion.span animate={{ rotate: open ? 45 : 0 }} transition={{ duration: 0.2 }} className="inline-flex">
              <Plus className="size-4" />
            </motion.span>
          </span>
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
            <p className="px-5 pb-6 leading-relaxed text-muted-foreground sm:px-6 sm:pr-16">{answer}</p>
          </motion.div>
        ) : null}
      </AnimatePresence>
    </li>
  );
}

export function Faq() {
  return (
    <section id="faq" aria-labelledby="faq-title" className="py-20 sm:py-28">
      <Container className="grid gap-10 lg:grid-cols-[0.8fr_1.6fr] lg:gap-16">
        <Reveal className="lg:sticky lg:top-28 lg:self-start">
          <p className="text-sm font-semibold text-accent-text">FAQ</p>
          <h2 id="faq-title" className="mt-4 text-3xl leading-[1.08] font-semibold tracking-[-0.03em] sm:text-5xl">
            Questions, answered
          </h2>
          <p className="mt-4 text-lg leading-relaxed text-muted-foreground">
            Something else on your mind?{" "}
            <a
              href={`${siteConfig.links.github}/discussions`}
              data-track="github_click"
              data-track-location="faq_discussions"
              target="_blank"
              rel="noreferrer"
              className={cn(link, "whitespace-nowrap")}
            >
              Ask on GitHub
            </a>
            .
          </p>
        </Reveal>
        <Reveal delay={0.08}>
          <ul className="space-y-3">
            {faqs.map((faq, i) => (
              <FaqItem key={faq.question} index={i} {...faq} />
            ))}
          </ul>
        </Reveal>
      </Container>
    </section>
  );
}
