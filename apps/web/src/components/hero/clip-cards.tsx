import { Code, Image as ImageIcon, Link2, Palette, Type } from "lucide-react";
import type { ComponentType, ReactNode, SVGProps } from "react";

import { cn } from "@/lib/cn";

function ClipCard({
  icon: Icon,
  kind,
  source,
  children,
  className,
}: {
  icon: ComponentType<SVGProps<SVGSVGElement>>;
  kind: string;
  source: string;
  children: ReactNode;
  className?: string;
}) {
  return (
    <div
      className={cn(
        "flex h-full flex-col rounded-2xl border border-border bg-surface p-4 shadow-lifted",
        className,
      )}
    >
      <div className="flex items-center justify-between text-xs text-muted-foreground">
        <span className="inline-flex items-center gap-1.5 font-medium text-foreground">
          <span className="inline-flex size-5 items-center justify-center rounded-md bg-accent-soft text-accent-text">
            <Icon className="size-3" aria-hidden="true" />
          </span>
          {kind}
        </span>
        <span>{source}</span>
      </div>
      <div className="mt-3 min-h-0 flex-1">{children}</div>
    </div>
  );
}

export function TextClip() {
  return (
    <ClipCard icon={Type} kind="Text" source="Notes · now">
      <p className="text-sm leading-snug">Standup moved to 10:30 tomorrow. Same room, same snacks.</p>
    </ClipCard>
  );
}

export function LinkClip() {
  return (
    <ClipCard icon={Link2} kind="Link" source="Browser · 2m">
      <div className="flex items-center gap-3 rounded-xl bg-surface-muted p-2.5">
        <span className="size-8 shrink-0 rounded-lg bg-linear-to-br from-accent to-[#8a4f1c]" />
        <span className="min-w-0">
          <span className="block truncate text-sm font-medium">Design review notes</span>
          <span className="block truncate text-xs text-muted-foreground">example.com/review</span>
        </span>
      </div>
    </ClipCard>
  );
}

export function ColorClip() {
  return (
    <ClipCard icon={Palette} kind="Color" source="Design · 5m">
      <div className="flex items-center gap-3">
        <span className="h-12 w-16 rounded-xl bg-[#d08a3a] ring-1 ring-black/5" />
        <span>
          <span className="block text-sm font-medium">Caramel</span>
          <span className="block font-mono text-xs text-muted-foreground">#D08A3A</span>
        </span>
      </div>
    </ClipCard>
  );
}

export function ImageClip() {
  return (
    <ClipCard icon={ImageIcon} kind="Image" source="Screenshot · 9m">
      <svg viewBox="0 0 200 60" className="h-14 w-full rounded-xl" aria-hidden="true" preserveAspectRatio="xMidYMid slice">
        <rect width="200" height="60" fill="#f2dcbc" />
        <circle cx="150" cy="18" r="9" fill="#e9a55a" />
        <path d="M0 60 45 22l30 24 25-16 45 30Z" fill="#b9773a" />
        <path d="M70 60l50-34 40 22 40-12v24Z" fill="#7a4a24" />
      </svg>
    </ClipCard>
  );
}

export function CodeClip() {
  return (
    <ClipCard icon={Code} kind="Code" source="Terminal · 14m">
      <pre className="overflow-hidden rounded-xl bg-[#22170f] p-2.5 font-mono text-[11px] leading-relaxed text-[#f3e9dc]">
        <span className="text-[#e8ae6b]">git</span> switch -c feat/pinboards
      </pre>
    </ClipCard>
  );
}
