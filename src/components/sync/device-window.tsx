"use client";

import { AnimatePresence, motion } from "motion/react";
import { Search } from "lucide-react";

import { cn } from "@/lib/cn";

export type SyncClip = { id: number; kind: string; label: string; swatch?: string };

export function DeviceWindow({
  platform,
  title,
  clips,
  badge,
}: {
  platform: "mac" | "windows";
  title: string;
  clips: SyncClip[];
  badge?: { clipId: number; text: string };
}) {
  return (
    <div className="overflow-hidden rounded-2xl border border-border bg-surface shadow-lifted">
      <div
        className={cn(
          "flex items-center gap-2 border-b border-border bg-surface-muted px-3 py-2.5 text-xs text-muted-foreground",
          platform === "windows" && "flex-row-reverse",
        )}
      >
        {platform === "mac" ? (
          <span className="flex gap-1.5">
            <span className="size-2.5 rounded-full bg-[#e0695a]" />
            <span className="size-2.5 rounded-full bg-[#e6b450]" />
            <span className="size-2.5 rounded-full bg-[#6fb35f]" />
          </span>
        ) : (
          <span className="flex items-center gap-3 text-[11px] leading-none">
            <span className="h-px w-2.5 bg-current" />
            <span className="size-2.5 border border-current" />
            <span>✕</span>
          </span>
        )}
        <span className={cn("flex-1 font-medium", platform === "mac" ? "text-center" : "text-left")}>{title}</span>
        {platform === "mac" ? <span className="w-[42px]" /> : null}
      </div>
      <div className="p-3">
        <div className="flex items-center gap-2 rounded-lg border border-border px-2.5 py-1.5 text-xs text-muted-foreground">
          <Search className="size-3.5" />
          Search your clipboard
        </div>
        <ul className="mt-3 space-y-2">
          <AnimatePresence initial={false}>
            {clips.map((clip) => (
              <motion.li
                key={clip.id}
                layout
                initial={{ opacity: 0, y: -12, scale: 0.97 }}
                animate={{ opacity: 1, y: 0, scale: 1 }}
                exit={{ opacity: 0, transition: { duration: 0.15 } }}
                transition={{ type: "spring", stiffness: 260, damping: 24 }}
                className="flex items-center gap-2.5 rounded-lg border border-border bg-background px-2.5 py-2"
              >
                {clip.swatch ? (
                  <span className="size-6 shrink-0 rounded-md" style={{ backgroundColor: clip.swatch }} />
                ) : (
                  <span className="inline-flex h-6 min-w-10 shrink-0 items-center justify-center rounded-md bg-accent-soft px-1.5 text-[10px] font-semibold text-accent-text">
                    {clip.kind}
                  </span>
                )}
                <span className="min-w-0 flex-1 truncate text-xs">{clip.label}</span>
                {badge?.clipId === clip.id ? (
                  <motion.span
                    initial={{ opacity: 0, scale: 0.8 }}
                    animate={{ opacity: 1, scale: 1 }}
                    className="rounded-full bg-accent px-2 py-0.5 text-[10px] font-semibold text-accent-foreground"
                  >
                    {badge.text}
                  </motion.span>
                ) : null}
              </motion.li>
            ))}
          </AnimatePresence>
        </ul>
      </div>
    </div>
  );
}
