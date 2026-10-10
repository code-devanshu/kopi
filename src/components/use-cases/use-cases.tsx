"use client";

import { AnimatePresence, motion } from "motion/react";
import { Check, Code2, GraduationCap, Palette, PenLine } from "lucide-react";
import { useRef, useState, type ComponentType, type KeyboardEvent, type SVGProps } from "react";

import { DesignerMock, DeveloperMock, StudentMock, WriterMock } from "@/components/use-cases/use-case-mocks";
import { Reveal } from "@/components/ui/reveal";
import { Section, SectionHeader } from "@/components/ui/section";
import { trackEvent } from "@/lib/analytics";
import { cn } from "@/lib/cn";

type UseCase = {
  id: string;
  label: string;
  icon: ComponentType<SVGProps<SVGSVGElement>>;
  title: string;
  description: string;
  points: string[];
  Mock: ComponentType;
};

const useCases: UseCase[] = [
  {
    id: "developers",
    label: "Developers",
    icon: Code2,
    title: "Commands, tokens and snippets, without the scrollback",
    description: "Stop digging through terminal history or chat threads for the command you ran yesterday.",
    points: ["Code keeps its formatting", "Pin the snippets you type every day", "Same history on your Mac laptop and Windows desktop"],
    Mock: DeveloperMock,
  },
  {
    id: "designers",
    label: "Designers",
    icon: Palette,
    title: "Colors, assets and specs in one place",
    description: "Copied hex codes show up as swatches, and images keep a preview so you can find them by sight.",
    points: ["Color values render as swatches", "Image clips with previews", "A board per project or client"],
    Mock: DesignerMock,
  },
  {
    id: "writers",
    label: "Writers",
    icon: PenLine,
    title: "Your best lines, ready when you are",
    description: "Keep bios, replies and quotes close, and paste them clean into any editor.",
    points: ["Paste as plain text by default if you like", "Pinboards for replies and bios", "Search across everything you've copied"],
    Mock: WriterMock,
  },
  {
    id: "students",
    label: "Students",
    icon: GraduationCap,
    title: "Research that doesn't fall through the cracks",
    description: "Collect citations, links and quotes as you read, then find them again when it's time to write.",
    points: ["A board per assignment", "Links keep their titles", "Free, with no account required"],
    Mock: StudentMock,
  },
];

export function UseCases() {
  const [active, setActive] = useState(0);
  const tabRefs = useRef<Array<HTMLButtonElement | null>>([]);
  const current = useCases[active];

  function select(index: number) {
    if (index === active) return;
    setActive(index);
    trackEvent("use_case_select", { use_case: useCases[index].id });
  }

  function onKeyDown(event: KeyboardEvent<HTMLDivElement>) {
    const keys: Record<string, number> = {
      ArrowRight: (active + 1) % useCases.length,
      ArrowLeft: (active - 1 + useCases.length) % useCases.length,
      Home: 0,
      End: useCases.length - 1,
    };
    const next = keys[event.key];
    if (next === undefined) return;
    event.preventDefault();
    select(next);
    tabRefs.current[next]?.focus();
  }

  return (
    <Section id="use-cases" labelledBy="use-cases-title" className="bg-surface-muted/50">
      <SectionHeader
        id="use-cases-title"
        title="Built for how you work"
        description="Whatever you copy all day, Kopi keeps it organised and easy to get back to."
      />

      <Reveal className="mt-12">
        <div
          role="tablist"
          aria-label="Use cases"
          onKeyDown={onKeyDown}
          className="flex w-fit max-w-full gap-1 overflow-x-auto rounded-full border border-border bg-surface p-1 shadow-soft"
        >
          {useCases.map((useCase, i) => {
            const selected = i === active;
            return (
              <button
                key={useCase.id}
                ref={(el) => {
                  tabRefs.current[i] = el;
                }}
                id={`tab-${useCase.id}`}
                type="button"
                role="tab"
                aria-selected={selected}
                aria-controls={`panel-${useCase.id}`}
                tabIndex={selected ? 0 : -1}
                onClick={() => select(i)}
                className={cn(
                  "relative inline-flex shrink-0 items-center gap-2 rounded-full px-3 py-2 text-sm font-medium transition-colors sm:px-4",
                  selected ? "text-accent-foreground" : "text-muted-foreground hover:text-foreground",
                )}
              >
                {selected ? (
                  <motion.span
                    layoutId="use-case-pill"
                    className="absolute inset-0 rounded-full bg-accent"
                    transition={{ type: "spring", stiffness: 380, damping: 32 }}
                  />
                ) : null}
                <useCase.icon className="relative size-4" aria-hidden="true" />
                <span className="relative">{useCase.label}</span>
              </button>
            );
          })}
        </div>

        <div
          id={`panel-${current.id}`}
          role="tabpanel"
          aria-labelledby={`tab-${current.id}`}
          tabIndex={0}
          className="mt-10 rounded-3xl"
        >
          <AnimatePresence mode="wait" initial={false}>
            <motion.div
              key={current.id}
              initial={{ opacity: 0, y: 12 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -8 }}
              transition={{ duration: 0.25, ease: "easeOut" }}
              className="grid items-center gap-10 rounded-3xl border border-border bg-surface p-6 shadow-soft sm:p-10 lg:grid-cols-2"
            >
              <div>
                <h3 className="text-2xl font-semibold">{current.title}</h3>
                <p className="mt-3 leading-relaxed text-muted-foreground">{current.description}</p>
                <ul className="mt-6 space-y-3">
                  {current.points.map((point) => (
                    <li key={point} className="flex items-center gap-3 text-sm">
                      <Check className="size-4 shrink-0 text-accent-text" aria-hidden="true" />
                      {point}
                    </li>
                  ))}
                </ul>
              </div>
              <current.Mock />
            </motion.div>
          </AnimatePresence>
        </div>
      </Reveal>
    </Section>
  );
}
