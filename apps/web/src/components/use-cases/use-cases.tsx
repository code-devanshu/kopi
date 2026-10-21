import { Code2, GraduationCap, Palette, PenLine } from "lucide-react";
import type { StaticImageData } from "next/image";
import type { ComponentType, SVGProps } from "react";

import codeDark from "@/assets/screens/preview-code-dark.png";
import codeLight from "@/assets/screens/preview-code-light.png";
import colorDark from "@/assets/screens/preview-color-dark.png";
import colorLight from "@/assets/screens/preview-color-light.png";
import filesDark from "@/assets/screens/preview-files-dark.png";
import filesLight from "@/assets/screens/preview-files-light.png";
import richDark from "@/assets/screens/preview-rich-dark.png";
import richLight from "@/assets/screens/preview-rich-light.png";
import { AppShot } from "@/components/ui/app-shot";
import { Reveal } from "@/components/ui/reveal";
import { Section, SectionHeader } from "@/components/ui/section";

type UseCase = {
  label: string;
  icon: ComponentType<SVGProps<SVGSVGElement>>;
  title: string;
  description: string;
  points: string[];
  /** The preview pane of Kopi's panel, cropped, with a clip of this kind selected. */
  shot: { light: StaticImageData; dark: StaticImageData; alt: string };
};

const useCases: UseCase[] = [
  {
    label: "Developers",
    icon: Code2,
    title: "Commands and code, without the scrollback",
    description: "Stop digging through terminal history or chat threads for the command you ran yesterday.",
    points: ["Code keeps its formatting", "Pin the commands you run daily", "One history on Mac and PC"],
    shot: { light: codeLight, dark: codeDark, alt: "Kopi previewing a Rust snippet copied from Zed." },
  },
  {
    label: "Designers",
    icon: Palette,
    title: "Colors and assets in one place",
    description: "Copied hex codes show up as swatches, and images keep a preview so you can find them by sight.",
    points: ["Colors as swatches", "Image previews", "A board per client"],
    shot: { light: colorLight, dark: colorDark, alt: "Kopi previewing the color #D08A3A, copied from Figma, as a swatch." },
  },
  {
    label: "Writers",
    icon: PenLine,
    title: "Your best lines, ready when you are",
    description: "Keep bios, replies and quotes close, and paste them clean into any editor.",
    points: ["Plain text with ⌥Return", "Snippets for replies", "Search everything you copied"],
    shot: { light: richLight, dark: richDark, alt: "Kopi previewing formatted text copied from Mail." },
  },
  {
    label: "Students",
    icon: GraduationCap,
    title: "Research that doesn't fall through the cracks",
    description: "Collect quotes, links and files as you read, then find them again when it's time to write.",
    points: ["A board per assignment", "Every clip knows its app", "Free, no account"],
    shot: { light: filesLight, dark: filesDark, alt: "Kopi previewing two files, a PDF and a notes file, copied from Finder." },
  },
];

export function UseCases() {
  return (
    <Section id="use-cases" labelledBy="use-cases-title" className="bg-surface-muted/50">
      <SectionHeader
        id="use-cases-title"
        align="center"
        title="Built for how you work"
        description="Whatever you copy all day, Kopi keeps it organised and easy to get back to."
      />

      <ul className="mt-14 grid gap-5 sm:mt-16 md:grid-cols-2">
        {useCases.map(({ label, icon: Icon, title, description, points, shot }, i) => (
          <Reveal
            as="li"
            key={label}
            delay={(i % 2) * 0.08}
            className="flex flex-col overflow-hidden rounded-3xl border border-border bg-surface shadow-soft"
          >
            <div className="p-7 sm:p-9">
              <p className="inline-flex items-center gap-2 text-sm font-semibold text-accent-text">
                <Icon className="size-4" aria-hidden="true" />
                {label}
              </p>
              <h3 className="mt-3 text-2xl leading-tight font-semibold tracking-tight">{title}</h3>
              <p className="mt-3 leading-relaxed text-muted-foreground">{description}</p>
              <ul className="mt-5 flex flex-wrap gap-2">
                {points.map((point) => (
                  <li key={point} className="rounded-full border border-border bg-background px-3 py-1 text-xs font-medium">
                    {point}
                  </li>
                ))}
              </ul>
            </div>

            {/* The preview rises from the bottom of the card, cut off by its edge. */}
            <div className="relative mt-auto h-56 overflow-hidden border-t border-border bg-[linear-gradient(160deg,var(--accent-soft),var(--surface-muted)_70%)] px-6 pt-8 sm:h-64 sm:px-12 sm:pt-10">
              <div className="mx-auto max-w-110 overflow-hidden rounded-t-2xl border border-b-0 border-border shadow-[0_2px_6px_rgb(var(--shadow-color)/0.08),0_24px_48px_-16px_rgb(var(--shadow-color)/0.35)]">
                <AppShot {...shot} sizes="440px" />
              </div>
            </div>
          </Reveal>
        ))}
      </ul>
    </Section>
  );
}
