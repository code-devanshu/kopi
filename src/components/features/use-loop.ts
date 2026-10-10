"use client";

import { useInView, useReducedMotion } from "motion/react";
import { useRef } from "react";

/** Looping illustrations only run while visible, and never when reduced motion is requested. */
export function useLoop<T extends Element = HTMLDivElement>() {
  const ref = useRef<T>(null);
  const inView = useInView(ref, { margin: "-40px" });
  const reduceMotion = useReducedMotion();
  return { ref, active: inView && !reduceMotion };
}
