"use client";

import { motion } from "motion/react";
import { Pin, Search } from "lucide-react";
import type { ReactNode } from "react";

import { useLoop } from "@/components/features/use-loop";

function Frame({ children, frameRef }: { children: ReactNode; frameRef: React.Ref<HTMLDivElement> }) {
  return (
    <div
      ref={frameRef}
      aria-hidden="true"
      className="relative h-32 overflow-hidden rounded-xl border border-border bg-surface-muted"
    >
      {children}
    </div>
  );
}

const historyRows = [72, 48, 86, 60, 40, 78, 54, 66];

export function HistoryIllustration() {
  const { ref, active } = useLoop();
  const rows = [...historyRows, ...historyRows];
  return (
    <Frame frameRef={ref}>
      <motion.div
        className="flex flex-col gap-2 p-3"
        animate={active ? { y: ["0%", "-50%"] } : { y: "0%" }}
        transition={{ duration: 14, ease: "linear", repeat: Infinity }}
      >
        {rows.map((width, i) => (
          <div key={i} className="flex items-center gap-2 rounded-lg bg-surface px-2 py-1.5 shadow-soft">
            <span className={i % 3 === 0 ? "size-3 rounded bg-accent" : "size-3 rounded bg-border"} />
            <span className="h-1.5 rounded-full bg-border" style={{ width: `${width}%` }} />
          </div>
        ))}
      </motion.div>
      <div className="pointer-events-none absolute inset-x-0 bottom-0 h-10 bg-linear-to-t from-surface-muted" />
    </Frame>
  );
}

export function SearchIllustration() {
  const { ref, active } = useLoop();
  return (
    <Frame frameRef={ref}>
      <div className="p-3">
        <div className="flex items-center gap-2 rounded-lg border border-border bg-surface px-2.5 py-1.5 text-xs">
          <Search className="size-3.5 text-muted-foreground" />
          <motion.span
            className="overflow-hidden whitespace-nowrap"
            animate={active ? { width: ["0ch", "7ch", "7ch", "0ch"] } : { width: "7ch" }}
            transition={{ duration: 4, times: [0, 0.35, 0.85, 1], repeat: Infinity, ease: "easeInOut" }}
          >
            invoice
          </motion.span>
          <span className="-ml-1.5 h-3.5 w-px bg-accent motion-safe:animate-pulse" />
        </div>
        <div className="mt-2 space-y-1.5">
          {["Invoice #2041, March", "invoice-template.docx"].map((label, i) => (
            <motion.div
              key={label}
              className={i === 0 ? "rounded-lg bg-accent-soft px-2.5 py-1.5 text-[11px]" : "rounded-lg px-2.5 py-1.5 text-[11px]"}
              animate={active ? { opacity: [0.25, 1, 1, 0.25] } : { opacity: 1 }}
              transition={{ duration: 4, times: [0, 0.4, 0.85, 1], repeat: Infinity, delay: i * 0.1 }}
            >
              {label}
            </motion.div>
          ))}
        </div>
      </div>
    </Frame>
  );
}

export function PinboardIllustration() {
  const { ref, active } = useLoop();
  return (
    <Frame frameRef={ref}>
      <div className="flex gap-1.5 border-b border-border px-3 pt-3 pb-2 text-[11px]">
        <span className="rounded-full bg-accent px-2 py-0.5 font-medium text-accent-foreground">Replies</span>
        <span className="rounded-full bg-surface px-2 py-0.5">Links</span>
        <span className="rounded-full bg-surface px-2 py-0.5">Colors</span>
      </div>
      <div className="grid grid-cols-2 gap-2 p-3">
        <div className="rounded-lg bg-surface p-2 text-[10px] leading-tight shadow-soft">Thanks! I&apos;ll take a look today.</div>
        <motion.div
          className="relative rounded-lg bg-surface p-2 text-[10px] leading-tight shadow-soft"
          animate={active ? { y: [-24, 0, 0, -24], opacity: [0, 1, 1, 0] } : { y: 0, opacity: 1 }}
          transition={{ duration: 4.5, times: [0, 0.2, 0.85, 1], repeat: Infinity, ease: "easeOut" }}
        >
          <Pin className="absolute -top-1.5 -right-1 size-3.5 rotate-12 fill-accent text-accent" />
          Happy to help. Here&apos;s the link.
        </motion.div>
      </div>
    </Frame>
  );
}

function MiniWindow({ label, controls }: { label: string; controls: "mac" | "win" }) {
  return (
    <div className="w-[38%] rounded-lg border border-border bg-surface p-2 shadow-soft">
      <div className={controls === "mac" ? "flex gap-1" : "flex justify-end gap-1"}>
        {controls === "mac" ? (
          <>
            <span className="size-1.5 rounded-full bg-[#e0695a]" />
            <span className="size-1.5 rounded-full bg-[#e6b450]" />
            <span className="size-1.5 rounded-full bg-[#6fb35f]" />
          </>
        ) : (
          <>
            <span className="h-0.5 w-1.5 self-center bg-muted-foreground" />
            <span className="size-1.5 border border-muted-foreground" />
            <span className="size-1.5 text-[7px] leading-[6px] text-muted-foreground">×</span>
          </>
        )}
      </div>
      <p className="mt-2 text-center text-[10px] font-medium">{label}</p>
    </div>
  );
}

export function SyncIllustration() {
  const { ref, active } = useLoop();
  return (
    <Frame frameRef={ref}>
      <div className="flex h-full items-center justify-between px-3">
        <MiniWindow label="Mac" controls="mac" />
        <div className="relative mx-2 h-px flex-1 border-t border-dashed border-muted-foreground/50">
          <motion.span
            className="absolute -top-[5px] left-0 size-2.5 rounded-sm bg-accent shadow-soft"
            animate={active ? { left: ["0%", "88%", "88%", "0%", "0%"] } : { left: "44%" }}
            transition={{ duration: 4, times: [0, 0.4, 0.5, 0.9, 1], repeat: Infinity, ease: "easeInOut" }}
          />
        </div>
        <MiniWindow label="Windows" controls="win" />
      </div>
    </Frame>
  );
}

export function SnippetsIllustration() {
  const { ref, active } = useLoop();
  const loop = { duration: 4, times: [0, 0.4, 0.5, 0.9, 1], repeat: Infinity };
  return (
    <Frame frameRef={ref}>
      <div className="grid h-full place-items-center px-3">
        <div className="w-full max-w-52 rounded-lg bg-surface p-2.5 shadow-soft">
          <p className="flex items-center gap-1.5 text-[11px] font-medium">
            Quick reply
            <span className="rounded bg-surface-muted px-1 font-mono text-[10px] text-muted-foreground">ty</span>
          </p>
          <div className="relative mt-1.5 h-5 text-[11px]">
            <motion.p
              className="absolute inset-0 leading-5"
              animate={active ? { opacity: [1, 1, 0, 0, 1] } : { opacity: 1 }}
              transition={loop}
            >
              Thanks, <span className="rounded bg-accent-soft px-1 font-mono text-[10px] text-accent-text">{"{input:Name}"}</span>!
            </motion.p>
            <motion.p
              className="absolute inset-0 leading-5"
              animate={active ? { opacity: [0, 0, 1, 1, 0] } : { opacity: 0 }}
              transition={loop}
            >
              Thanks, <span className="font-medium text-accent-text">Priya</span>!
            </motion.p>
          </div>
        </div>
      </div>
    </Frame>
  );
}

function Key({ children, index, active }: { children: ReactNode; index: number; active: boolean }) {
  return (
    <motion.kbd
      className="inline-flex h-10 min-w-10 items-center justify-center rounded-lg border border-border bg-surface px-2.5 font-sans text-sm font-medium shadow-[0_3px_0_var(--border)]"
      animate={
        active
          ? { y: [0, 0, 3, 3, 0], boxShadow: ["0 3px 0 var(--border)", "0 3px 0 var(--border)", "0 0 0 var(--border)", "0 0 0 var(--border)", "0 3px 0 var(--border)"] }
          : undefined
      }
      transition={{ duration: 2.8, times: [0, 0.1 + index * 0.12, 0.2 + index * 0.12, 0.7, 0.8], repeat: Infinity }}
    >
      {children}
    </motion.kbd>
  );
}

export function KeyboardIllustration() {
  const { ref, active } = useLoop();
  return (
    <Frame frameRef={ref}>
      <div className="flex h-full items-center justify-center gap-2">
        <Key index={0} active={active}>⌘ / Ctrl</Key>
        <Key index={1} active={active}>⇧</Key>
        <Key index={2} active={active}>V</Key>
      </div>
    </Frame>
  );
}
