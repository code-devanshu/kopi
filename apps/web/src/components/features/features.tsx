"use client";

import { motion } from "motion/react";
import { History, Keyboard, Pin, RefreshCw, Search, Type } from "lucide-react";
import type { ComponentType, SVGProps } from "react";

import {
  HistoryIllustration,
  KeyboardIllustration,
  PinboardIllustration,
  PlainTextIllustration,
  SearchIllustration,
  SyncIllustration,
} from "@/components/features/feature-illustrations";
import { Section, SectionHeader } from "@/components/ui/section";

type Feature = {
  title: string;
  description: string;
  icon: ComponentType<SVGProps<SVGSVGElement>>;
  Illustration: ComponentType;
};

const features: Feature[] = [
  {
    title: "Unlimited clipboard history",
    description: "Text, links, images, colors and code are kept as you copy them, so nothing gets lost to the next copy.",
    icon: History,
    Illustration: HistoryIllustration,
  },
  {
    title: "Instant search",
    description: "Type a few letters to find something you copied last week. Narrow it down by type or by the app it came from.",
    icon: Search,
    Illustration: SearchIllustration,
  },
  {
    title: "Pinboards",
    description: "Keep the replies, addresses and snippets you reuse in boards that stay put, one shortcut away.",
    icon: Pin,
    Illustration: PinboardIllustration,
  },
  {
    title: "Mac ↔ Windows sync",
    description: "Copy on one computer and paste on the other, over your local network, end-to-end encrypted and entirely optional.",
    icon: RefreshCw,
    Illustration: SyncIllustration,
  },
  {
    title: "Paste as plain text",
    description: "Drop the fonts, colors and stray formatting so whatever you paste matches where it lands.",
    icon: Type,
    Illustration: PlainTextIllustration,
  },
  {
    title: "Keyboard-first",
    description: "Open Kopi, search, pick and paste without reaching for the mouse. Every shortcut can be changed.",
    icon: Keyboard,
    Illustration: KeyboardIllustration,
  },
];

export function Features() {
  return (
    <Section id="features" labelledBy="features-title">
      <SectionHeader
        id="features-title"
        title="A clipboard that keeps up with you"
        description="Kopi runs in the background and keeps everything you copy, ready to paste again."
      />
      <ul className="mt-14 grid gap-5 sm:grid-cols-2 lg:grid-cols-3">
        {features.map(({ title, description, icon: Icon, Illustration }, i) => (
          <motion.li
            key={title}
            initial={{ opacity: 0, y: 24 }}
            whileInView={{ opacity: 1, y: 0 }}
            whileHover={{ y: -4 }}
            viewport={{ once: true, margin: "0px 0px -60px 0px" }}
            transition={{ duration: 0.5, delay: (i % 3) * 0.08, ease: [0.22, 1, 0.36, 1] }}
            className="flex flex-col rounded-2xl border border-border bg-surface p-5 shadow-soft transition-shadow hover:shadow-lifted"
          >
            <Illustration />
            <div className="mt-5 flex items-center gap-3">
              <span className="inline-flex size-9 items-center justify-center rounded-xl bg-accent-soft text-accent-text">
                <Icon className="size-[18px]" aria-hidden="true" />
              </span>
              <h3 className="text-lg font-semibold">{title}</h3>
            </div>
            <p className="mt-3 leading-relaxed text-muted-foreground">{description}</p>
          </motion.li>
        ))}
      </ul>
    </Section>
  );
}
