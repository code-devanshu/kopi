import { Check, Keyboard, Layers, Pin, ScanText, Search, SunMoon, TextQuote, Type } from "lucide-react";
import type { StaticImageData } from "next/image";
import type { ComponentType, SVGProps } from "react";

import boardDark from "@/assets/screens/board-dark.png";
import boardLight from "@/assets/screens/board-light.png";
import imagesDark from "@/assets/screens/images-dark.png";
import imagesLight from "@/assets/screens/images-light.png";
import searchDark from "@/assets/screens/search-dark.png";
import searchLight from "@/assets/screens/search-light.png";
import snippetsDark from "@/assets/screens/snippets-dark.png";
import snippetsLight from "@/assets/screens/snippets-light.png";
import { AppShot } from "@/components/ui/app-shot";
import { Reveal } from "@/components/ui/reveal";
import { Section, SectionHeader } from "@/components/ui/section";
import { cn } from "@/lib/cn";

type Icon = ComponentType<SVGProps<SVGSVGElement>>;

type Feature = {
  eyebrow: string;
  icon: Icon;
  title: string;
  description: string;
  points: string[];
  shot: { light: StaticImageData; dark: StaticImageData; alt: string };
};

const features: Feature[] = [
  {
    eyebrow: "Search",
    icon: Search,
    title: "Find it in a few letters.",
    description: "Type part of anything you copied, last minute or last month, and it's there. Narrow it down by kind or by the app it came from.",
    points: ["Matches highlighted as you type", "Filters for kind and source app", "Unlimited history, unless you set a limit"],
    shot: {
      light: searchLight,
      dark: searchDark,
      alt: "Kopi's panel searching for “invoice”, with the word highlighted in a code snippet, an email and several notes.",
    },
  },
  {
    eyebrow: "Text in images",
    icon: ScanText,
    title: "Search your screenshots.",
    description: "Kopi reads the words in the screenshots and images you copy, so the booking reference, the error message or the slide from a call turns up when you search for it. It's read on your computer and never uploaded.",
    points: [
      "Matches highlighted in the image's text",
      "Copy just the text from any image",
      "Uses the text recognition built into macOS and Windows",
      "Turn it off in Settings whenever you like",
    ],
    shot: {
      light: imagesLight,
      dark: imagesDark,
      alt: "Kopi's panel searching for “lisbon”. The top result is a screenshot of a flight booking, found by the words in it, and the preview shows the text Kopi read from the image with a Copy text button.",
    },
  },
  {
    eyebrow: "Snippets",
    icon: TextQuote,
    title: "Write it once. Paste it filled in.",
    description: "Save the replies, signatures and commands you type again and again. Placeholders fill in the date, your last copy or a name as you paste.",
    points: ["Type its keyword to bring it to the top", "Placeholders for the date, a clip or your input", "Put the cursor where you'll type next"],
    shot: {
      light: snippetsLight,
      dark: snippetsDark,
      alt: "Kopi's Snippets tab, previewing a reply whose name, date and tracking link fill in when it's pasted.",
    },
  },
  {
    eyebrow: "Pinboards",
    icon: Pin,
    title: "Keep what you reuse close.",
    description: "Pin a clip so it never ages out, or gather clips into boards for a project, a client or a habit. Each board is one click from the search box.",
    points: ["Boards with your own names and colors", "Pinned clips are never removed by limits", "Drag clips onto a board"],
    shot: {
      light: boardLight,
      dark: boardDark,
      alt: "Kopi's panel showing a pinboard called Work, with a link, an invoice note and a git command.",
    },
  },
];

const extras: { icon: Icon; title: string; description: string }[] = [
  { icon: Keyboard, title: "Keyboard first", description: "Arrows, Return, and ⌘1–9 to paste without the mouse." },
  { icon: Type, title: "Paste as plain text", description: "⌥Return pastes the words without their formatting." },
  { icon: Layers, title: "Every kind of copy", description: "Text, rich text, links, images, files, colors and code." },
  { icon: SunMoon, title: "At home on both", description: "Native shortcuts, light and dark, on Mac and Windows." },
];

function FeatureRow({ feature, index }: { feature: Feature; index: number }) {
  const { eyebrow, icon: Icon, title, description, points, shot } = feature;
  const flip = index % 2 === 1;
  return (
    <li className={cn("grid items-center gap-10 lg:gap-14", flip ? "lg:grid-cols-[1.25fr_0.75fr]" : "lg:grid-cols-[0.75fr_1.25fr]")}>
      <Reveal className={cn(flip && "lg:order-2")}>
        <p className="inline-flex items-center gap-2 text-sm font-semibold text-accent-text">
          <Icon className="size-4" aria-hidden="true" />
          {eyebrow}
        </p>
        <h3 className="mt-4 text-3xl leading-tight font-semibold tracking-[-0.03em] sm:text-4xl">{title}</h3>
        <p className="mt-4 text-lg leading-relaxed text-muted-foreground">{description}</p>
        <ul className="mt-6 space-y-2.5">
          {points.map((point) => (
            <li key={point} className="flex gap-2.5 text-sm text-foreground">
              <Check className="mt-0.5 size-4 shrink-0 text-accent-text" aria-hidden="true" />
              {point}
            </li>
          ))}
        </ul>
      </Reveal>

      {/* The shot sits on a tinted stage, cropped at the far edge so it reads as a window, not a figure. */}
      <Reveal delay={0.1} className={cn(flip && "lg:order-1")}>
        <div className="relative overflow-hidden rounded-3xl border border-border bg-[linear-gradient(135deg,var(--accent-soft),var(--surface-muted)_60%)] p-3 sm:p-5">
          <div className="rounded-[14px] shadow-[0_2px_6px_rgb(var(--shadow-color)/0.08),0_30px_60px_-20px_rgb(var(--shadow-color)/0.4)]">
            <AppShot {...shot} sizes="(min-width: 1024px) 680px, calc(100vw - 2rem)" />
          </div>
        </div>
      </Reveal>
    </li>
  );
}

export function Features() {
  return (
    <Section id="features" labelledBy="features-title">
      <SectionHeader
        id="features-title"
        align="center"
        title="A clipboard that keeps up with you"
        description="Kopi runs quietly in the background and keeps everything you copy, ready to paste again."
      />

      <ul className="mt-16 space-y-24 sm:mt-20 sm:space-y-32">
        {features.map((feature, i) => (
          <FeatureRow key={feature.eyebrow} feature={feature} index={i} />
        ))}
      </ul>

      <ul className="mt-24 grid gap-x-8 gap-y-10 border-t border-border pt-12 sm:mt-32 sm:grid-cols-2 lg:grid-cols-4">
        {extras.map(({ icon: Icon, title, description }, i) => (
          <Reveal as="li" key={title} delay={i * 0.06}>
            <Icon className="size-5 text-accent-text" aria-hidden="true" />
            <h3 className="mt-3 font-semibold">{title}</h3>
            <p className="mt-1.5 text-sm leading-relaxed text-muted-foreground">{description}</p>
          </Reveal>
        ))}
      </ul>
    </Section>
  );
}
