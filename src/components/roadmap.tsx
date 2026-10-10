import { Check } from "lucide-react";

import { Reveal } from "@/components/ui/reveal";
import { Section, SectionHeader } from "@/components/ui/section";
import { cn } from "@/lib/cn";

type Stage = { title: string; description: string; status: "done" | "current" | "next" };

// TODO: Update statuses as the project moves forward. Exactly one stage should be "current".
const stages: Stage[] = [
  { title: "Design", description: "Shaping the experience and the sync protocol in the open.", status: "current" },
  { title: "Mac alpha", description: "First builds for macOS, shared with early testers.", status: "next" },
  { title: "Windows alpha", description: "The same app, native on Windows.", status: "next" },
  { title: "Sync beta", description: "End-to-end encrypted sync between your computers.", status: "next" },
  { title: "1.0", description: "A stable release on both platforms, free for everyone.", status: "next" },
];

export function Roadmap() {
  return (
    <Section id="roadmap" labelledBy="roadmap-title">
      <SectionHeader
        id="roadmap-title"
        title="Where Kopi is today"
        description="No launch date yet. Here's the plan, and the step we're on right now."
      />
      <div className="relative mt-14">
        <span
          aria-hidden="true"
          className="absolute top-2 bottom-2 left-[15px] w-px bg-border lg:top-[15px] lg:right-[10%] lg:bottom-auto lg:left-[10%] lg:h-px lg:w-auto"
        />
        <ol className="relative grid gap-8 lg:grid-cols-5 lg:gap-4">
          {stages.map((stage, i) => (
            <Reveal
              as="li"
              key={stage.title}
              delay={i * 0.07}
              className="relative flex gap-4 lg:flex-col lg:items-center lg:text-center"
            >
              <span
                className={cn(
                  "relative z-10 inline-flex size-8 shrink-0 items-center justify-center rounded-full border text-xs font-semibold",
                  stage.status === "done" && "border-accent bg-accent text-accent-foreground",
                  stage.status === "current" && "border-accent bg-background text-accent-text ring-4 ring-accent-soft",
                  stage.status === "next" && "border-border bg-background text-muted-foreground",
                )}
              >
                {stage.status === "done" ? <Check className="size-4" aria-hidden="true" /> : i + 1}
              </span>
              <div>
                <h3 className="flex flex-wrap items-center gap-2 font-semibold lg:justify-center">
                  {stage.title}
                  {stage.status === "current" ? (
                    <span className="rounded-full bg-accent-soft px-2 py-0.5 text-xs font-medium text-accent-text">
                      We&apos;re here
                    </span>
                  ) : null}
                </h3>
                <p className="mt-1 text-sm leading-relaxed text-muted-foreground">{stage.description}</p>
                <span className="sr-only">
                  {stage.status === "done" ? "Completed" : stage.status === "current" ? "In progress" : "Upcoming"}
                </span>
              </div>
            </Reveal>
          ))}
        </ol>
      </div>
    </Section>
  );
}
