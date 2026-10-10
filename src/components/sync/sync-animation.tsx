"use client";

import { motion, useInView, useReducedMotion } from "motion/react";
import { Lock } from "lucide-react";
import { useEffect, useRef, useState } from "react";

import { DeviceWindow, type SyncClip } from "@/components/sync/device-window";

const incoming: Omit<SyncClip, "id">[] = [
  { kind: "Text", label: "Flight lands 18:40, terminal 2" },
  { kind: "Link", label: "example.com/project-brief" },
  { kind: "Color", label: "Caramel  #D08A3A", swatch: "#d08a3a" },
  { kind: "Code", label: "npm run dev -- --port 4000" },
];

const history: SyncClip[] = [
  { id: -1, kind: "Text", label: "Meeting notes, draft 3" },
  { id: -2, kind: "Email", label: "hello@example.com" },
];

const VISIBLE = 3;
const STEP_MS = 1500;

function clipsUpTo(count: number): SyncClip[] {
  const fresh = Array.from({ length: count }, (_, i) => ({ id: i, ...incoming[i % incoming.length] })).reverse();
  return [...fresh, ...history].slice(0, VISIBLE);
}

/**
 * A three-step loop: copy on the Mac → encrypted clip travels → it lands on Windows.
 * `step` keeps counting so every clip gets a fresh key and list animations stay smooth.
 */
export function SyncAnimation() {
  const ref = useRef<HTMLDivElement>(null);
  const inView = useInView(ref, { margin: "-80px" });
  const reduceMotion = useReducedMotion();
  const [step, setStep] = useState(2);

  useEffect(() => {
    if (!inView || reduceMotion) return;
    const timer = setInterval(() => setStep((s) => s + 1), STEP_MS);
    return () => clearInterval(timer);
  }, [inView, reduceMotion]);

  const round = Math.floor(step / 3);
  const phase = step % 3; // 0 copied on Mac, 1 travelling, 2 arrived on Windows
  const macClips = clipsUpTo(round + 1);
  const windowsClips = clipsUpTo(phase === 2 ? round + 1 : round);
  const travelling = incoming[round % incoming.length];

  return (
    <div
      ref={ref}
      role="img"
      aria-label="Animation: a clip copied on a Mac travels, encrypted, to a Windows PC and appears in its clipboard history."
      className="grid gap-0 md:grid-cols-[1fr_7rem_1fr] md:items-center"
    >
      <div aria-hidden="true">
        <DeviceWindow
          platform="mac"
          title="Kopi"
          clips={macClips}
          badge={phase === 0 ? { clipId: round, text: "Copied" } : undefined}
        />
      </div>

      <div aria-hidden="true" className="relative mx-auto h-24 w-px md:h-px md:w-full">
        <div className="absolute inset-0 border-l border-dashed border-muted-foreground/40 md:border-t md:border-l-0" />
        <span className="absolute top-1/2 left-1/2 z-10 inline-flex -translate-x-1/2 -translate-y-1/2 items-center gap-1 rounded-full border border-border bg-surface px-2 py-1 text-[10px] font-medium whitespace-nowrap text-muted-foreground shadow-soft">
          <Lock className="size-3 text-accent-text" />
          E2E
        </span>
        {phase === 1 && !reduceMotion ? (
          <>
            <motion.span
              key={`v-${step}`}
              className="absolute -left-[22px] z-20 rounded-md bg-accent px-2 py-1 text-[10px] font-semibold text-accent-foreground shadow-lifted md:hidden"
              initial={{ top: "-10%", opacity: 0 }}
              animate={{ top: ["-10%", "80%"], opacity: [0, 1, 1, 0] }}
              transition={{ duration: STEP_MS / 1000, ease: "easeInOut" }}
            >
              {travelling.kind}
            </motion.span>
            <motion.span
              key={`h-${step}`}
              className="absolute -top-3 z-20 hidden rounded-md bg-accent px-2 py-1 text-[10px] font-semibold text-accent-foreground shadow-lifted md:block"
              initial={{ left: "-10%", opacity: 0 }}
              animate={{ left: ["-10%", "80%"], opacity: [0, 1, 1, 0] }}
              transition={{ duration: STEP_MS / 1000, ease: "easeInOut" }}
            >
              {travelling.kind}
            </motion.span>
          </>
        ) : null}
      </div>

      <div aria-hidden="true">
        <DeviceWindow
          platform="windows"
          title="Kopi"
          clips={windowsClips}
          badge={phase === 2 ? { clipId: round, text: "Synced" } : undefined}
        />
      </div>
    </div>
  );
}
