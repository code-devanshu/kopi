import { ArrowUpRight, Check } from "lucide-react";

import { Reveal } from "@/components/ui/reveal";
import { Section } from "@/components/ui/section";
import { siteConfig } from "@/lib/site";

type Stage = { title: string; description: string; status: "done" | "current" | "next"; points?: string[] };

// TODO: Update statuses as the project moves forward. Exactly one stage should be "current".
// Keep these in step with ROADMAP.md at the repository root.
const stages: Stage[] = [
  { title: "Website & waitlist", description: "A home for the project and a way to follow along.", status: "done" },
  { title: "Design", description: "Shaping the app and its architecture in the open.", status: "done" },
  { title: "Mac alpha", description: "First builds for macOS, out for early testers.", status: "done" },
  { title: "Encrypted sync", description: "Optional, end-to-end encrypted sync between your computers on the same network.", status: "done" },
  {
    title: "Windows alpha",
    description: "The same app, native on Windows 10 and 11, out now for early testers.",
    status: "current",
    points: ["Feature parity with the Mac, checked by testers", "Fixes from testers' reports"],
  },
  { title: "Private beta", description: "More testers on both platforms, features rounded out.", status: "next" },
  { title: "1.0", description: "A stable release on both platforms, free for everyone.", status: "next" },
];

const done = stages.filter((s) => s.status === "done");
const current = stages.find((s) => s.status === "current");
const upcoming = stages.filter((s) => s.status === "next");
// Halfway through the current stage.
const progress = ((done.length + (current ? 0.5 : 0)) / stages.length) * 100;

function ColumnTitle({ children, count }: { children: string; count: number }) {
  return (
    <h3 className="flex items-center justify-between text-sm font-semibold">
      {children}
      <span className="text-xs font-medium text-muted-foreground">{count}</span>
    </h3>
  );
}

export function Roadmap() {
  return (
    <Section id="roadmap" labelledBy="roadmap-title" className="bg-surface-muted/50">
      <Reveal className="flex flex-col justify-between gap-6 md:flex-row md:items-end">
        <div className="max-w-2xl">
          <p className="text-sm font-semibold text-accent-text">Roadmap</p>
          <h2 id="roadmap-title" className="mt-4 text-3xl leading-[1.08] font-semibold tracking-[-0.03em] sm:text-5xl">
            Where Kopi is today
          </h2>
          <p className="mt-4 text-lg leading-relaxed text-muted-foreground">
            No dates, just the plan and the step we&apos;re on right now.
          </p>
        </div>
        <a
          href={`${siteConfig.links.github}/blob/main/ROADMAP.md`}
          data-track="github_click"
          data-track-location="roadmap"
          target="_blank"
          rel="noreferrer"
          className="inline-flex shrink-0 items-center gap-1.5 text-sm font-medium text-accent-text underline-offset-4 hover:underline"
        >
          The full roadmap
          <ArrowUpRight className="size-4" aria-hidden="true" />
        </a>
      </Reveal>

      <Reveal delay={0.05} className="mt-10">
        <div
          role="progressbar"
          aria-label="Progress to 1.0"
          aria-valuemin={0}
          aria-valuemax={stages.length}
          aria-valuenow={done.length}
          aria-valuetext={`${done.length} of ${stages.length} stages done`}
          className="h-2 overflow-hidden rounded-full bg-border"
        >
          <div className="h-full rounded-full bg-accent" style={{ width: `${progress}%` }} />
        </div>
        <p className="mt-3 text-sm text-muted-foreground">
          {done.length} of {stages.length} stages done
          {current ? <>, {current.title} in progress</> : null}
        </p>
      </Reveal>

      <div className="mt-10 grid gap-5 lg:grid-cols-3">
        <Reveal className="rounded-3xl border border-border bg-surface p-6 sm:p-7">
          <ColumnTitle count={done.length}>Done</ColumnTitle>
          <ol className="mt-5 space-y-4">
            {done.map((stage) => (
              <li key={stage.title} className="flex gap-3">
                <span className="mt-0.5 inline-flex size-5 shrink-0 items-center justify-center rounded-full bg-accent text-accent-foreground">
                  <Check className="size-3" strokeWidth={3} aria-hidden="true" />
                </span>
                <div>
                  <p className="font-medium">{stage.title}</p>
                  <p className="mt-0.5 text-sm leading-relaxed text-muted-foreground">{stage.description}</p>
                </div>
              </li>
            ))}
          </ol>
        </Reveal>

        {current ? (
          <Reveal
            delay={0.08}
            className="relative overflow-hidden rounded-3xl border border-accent bg-[linear-gradient(160deg,var(--accent-soft),var(--surface)_75%)] p-6 shadow-lifted sm:p-7"
          >
            <ColumnTitle count={1}>Now</ColumnTitle>
            <p className="mt-5 inline-flex items-center gap-2 rounded-full bg-surface px-2.5 py-1 text-xs font-medium text-accent-text shadow-soft">
              <span className="relative flex size-2" aria-hidden="true">
                <span className="absolute inline-flex size-full animate-ping rounded-full bg-accent opacity-60 motion-reduce:animate-none" />
                <span className="relative inline-flex size-2 rounded-full bg-accent" />
              </span>
              In progress
            </p>
            <p className="mt-4 text-3xl font-semibold tracking-[-0.03em]">{current.title}</p>
            <p className="mt-2 leading-relaxed text-muted-foreground">{current.description}</p>
            {current.points ? (
              <ul className="mt-6 space-y-2.5 border-t border-border pt-5">
                {current.points.map((point) => (
                  <li key={point} className="flex gap-2.5 text-sm">
                    <span className="mt-2 size-1.5 shrink-0 rounded-full bg-accent" aria-hidden="true" />
                    {point}
                  </li>
                ))}
              </ul>
            ) : null}
            <a
              href={`${siteConfig.links.github}/issues/new/choose`}
              data-track="github_click"
              data-track-location="roadmap_issues"
              target="_blank"
              rel="noreferrer"
              className="mt-6 inline-flex items-center gap-1.5 text-sm font-medium text-accent-text underline-offset-4 hover:underline"
            >
              Try it and tell us what you find
              <ArrowUpRight className="size-4" aria-hidden="true" />
            </a>
          </Reveal>
        ) : null}

        <Reveal delay={0.16} className="rounded-3xl border border-dashed border-border p-6 sm:p-7">
          <ColumnTitle count={upcoming.length}>Next</ColumnTitle>
          <ol className="mt-5 space-y-4">
            {upcoming.map((stage, i) => (
              <li key={stage.title} className="flex gap-3">
                <span className="mt-0.5 inline-flex size-5 shrink-0 items-center justify-center rounded-full border border-border text-[10px] font-semibold text-muted-foreground">
                  {done.length + (current ? 1 : 0) + i + 1}
                </span>
                <div>
                  <p className="font-medium">{stage.title}</p>
                  <p className="mt-0.5 text-sm leading-relaxed text-muted-foreground">{stage.description}</p>
                </div>
              </li>
            ))}
          </ol>
        </Reveal>
      </div>
    </Section>
  );
}
