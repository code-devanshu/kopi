"use client";

import { motion } from "motion/react";

export function SuccessMessage({ onReset }: { onReset: () => void }) {
  return (
    <motion.div
      role="status"
      initial={{ opacity: 0, scale: 0.96 }}
      animate={{ opacity: 1, scale: 1 }}
      transition={{ type: "spring", stiffness: 200, damping: 20 }}
      className="flex flex-col items-center py-10 text-center"
    >
      <svg viewBox="0 0 64 64" className="size-20" aria-hidden="true">
        <motion.circle
          cx="32"
          cy="32"
          r="28"
          fill="var(--accent-soft)"
          stroke="var(--accent)"
          strokeWidth="3"
          initial={{ pathLength: 0 }}
          animate={{ pathLength: 1 }}
          transition={{ duration: 0.6, ease: "easeOut" }}
        />
        <motion.path
          d="M20 33.5 28 41l16-17"
          fill="none"
          stroke="var(--accent-text)"
          strokeWidth="4"
          strokeLinecap="round"
          strokeLinejoin="round"
          initial={{ pathLength: 0 }}
          animate={{ pathLength: 1 }}
          transition={{ duration: 0.4, delay: 0.45, ease: "easeOut" }}
        />
      </svg>
      <motion.div initial={{ opacity: 0, y: 8 }} animate={{ opacity: 1, y: 0 }} transition={{ delay: 0.6 }}>
        <h3 className="mt-6 text-2xl font-semibold">You&apos;re on the list</h3>
        <p className="mt-2 max-w-sm text-muted-foreground">
          Thanks for the interest. We&apos;ll be in touch when there&apos;s something for you to try.
        </p>
        <button
          type="button"
          onClick={onReset}
          className="mt-6 text-sm font-medium text-accent-text underline underline-offset-4 hover:no-underline"
        >
          Add someone else
        </button>
      </motion.div>
    </motion.div>
  );
}
