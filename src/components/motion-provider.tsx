"use client";

import { MotionConfig } from "motion/react";
import type { ReactNode } from "react";

/** Honors prefers-reduced-motion for every Motion component in the tree. */
export function MotionProvider({ children }: { children: ReactNode }) {
  return <MotionConfig reducedMotion="user">{children}</MotionConfig>;
}
