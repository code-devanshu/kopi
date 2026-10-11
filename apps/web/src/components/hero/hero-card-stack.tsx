"use client";

import { motion, useReducedMotion } from "motion/react";
import type { ComponentType } from "react";

import { CodeClip, ColorClip, ImageClip, LinkClip, TextClip } from "@/components/hero/clip-cards";

type CardSpec = {
  Card: ComponentType;
  x: number;
  y: number;
  rotate: number;
  drift: number;
};

// Positions are offsets from the centre of a 448px-wide stage.
const cards: CardSpec[] = [
  { Card: CodeClip, x: 10, y: 150, rotate: -2, drift: 6.5 },
  { Card: ImageClip, x: 100, y: 55, rotate: 4, drift: 7 },
  { Card: ColorClip, x: -100, y: 25, rotate: -4, drift: 5.5 },
  { Card: LinkClip, x: 90, y: -80, rotate: 5, drift: 6 },
  { Card: TextClip, x: -80, y: -140, rotate: -6, drift: 5 },
];

export function HeroCardStack() {
  const reduceMotion = useReducedMotion();

  return (
    <div className="relative h-[400px] w-full sm:h-[480px]" aria-hidden="true">
      <div className="absolute top-1/2 left-1/2 h-[480px] w-[448px] -translate-x-1/2 -translate-y-1/2 scale-[0.8] sm:scale-100">
        {cards.map(({ Card, x, y, rotate, drift }, index) => (
          <motion.div
            key={index}
            className="absolute top-1/2 left-1/2 -mt-[58px] -ml-[120px] h-[116px] w-[240px]"
            initial={{ x: 0, y: 30, rotate: 0, opacity: 0, scale: 0.92 }}
            animate={{ x, y, rotate, opacity: 1, scale: 1 }}
            transition={{
              delay: 0.25 + (cards.length - index) * 0.08,
              type: "spring",
              stiffness: 120,
              damping: 18,
            }}
          >
            <motion.div
              className="h-full"
              animate={reduceMotion ? undefined : { y: [0, -8, 0], rotate: [0, index % 2 ? 0.8 : -0.8, 0] }}
              transition={{
                duration: drift,
                repeat: Infinity,
                ease: "easeInOut",
                delay: 1.2 + index * 0.3,
              }}
            >
              <Card />
            </motion.div>
          </motion.div>
        ))}
      </div>
    </div>
  );
}
