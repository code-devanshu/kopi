"use client";

import { motion, useReducedMotion } from "motion/react";

import panelDark from "@/assets/screens/panel-dark.png";
import panelLight from "@/assets/screens/panel-light.png";
import { AppShot } from "@/components/ui/app-shot";

/** The panel, as ⇧⌘V opens it, rising into place under the headline. */
export function HeroShot() {
  const reduceMotion = useReducedMotion();

  return (
    <motion.div
      className="relative mx-auto max-w-5xl"
      // No fade: the shot is the page's largest image, so it shows before the script runs.
      initial={reduceMotion ? false : { y: 40, scale: 0.97 }}
      animate={{ y: 0, scale: 1 }}
      transition={{ delay: 0.3, duration: 0.9, ease: [0.22, 1, 0.36, 1] }}
    >
      {/* A caramel glow behind the panel, as if it lit the page. */}
      <div
        aria-hidden="true"
        className="absolute -inset-x-10 -top-10 bottom-0 -z-10 rounded-[3rem] bg-[radial-gradient(50%_55%_at_50%_45%,color-mix(in_oklab,var(--accent)_35%,transparent),transparent_70%)] blur-2xl"
      />
      <div className="rounded-[18px] shadow-[0_2px_6px_rgb(var(--shadow-color)/0.08),0_40px_80px_-24px_rgb(var(--shadow-color)/0.45)] sm:rounded-[22px]">
        <AppShot
          light={panelLight}
          dark={panelDark}
          alt="Kopi's panel: a search box, tabs for history, pins, snippets and pinboards, a list of recent clips with a selected snippet of code, and its preview."
          sizes="(min-width: 1088px) 1024px, calc(100vw - 2rem)"
          important
        />
      </div>
      <p className="mt-6 text-center text-sm text-muted-foreground">
        Press{" "}
        <kbd className="rounded-md border border-border bg-surface px-1.5 py-0.5 font-sans text-xs text-foreground shadow-soft">
          ⇧⌘V
        </kbd>{" "}
        or{" "}
        <kbd className="rounded-md border border-border bg-surface px-1.5 py-0.5 font-sans text-xs text-foreground shadow-soft">
          Ctrl+Shift+V
        </kbd>{" "}
        in any app.
      </p>
    </motion.div>
  );
}
