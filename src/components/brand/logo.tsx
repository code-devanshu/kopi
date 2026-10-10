import type { SVGProps } from "react";

import { cn } from "@/lib/cn";

/** Kopi mark: two stacked cards, the front one shaped like a cup. Keep in sync with app/icon.svg. */
export function LogoMark(props: SVGProps<SVGSVGElement>) {
  return (
    <svg viewBox="0 0 32 32" aria-hidden="true" {...props}>
      <rect width="32" height="32" rx="8" fill="#2B1D14" />
      <rect x="6.5" y="6" width="13" height="14" rx="3" fill="#D08A3A" />
      <rect x="10.5" y="11" width="13" height="15" rx="3" fill="#FAF6EF" />
      <path
        d="M23.5 14.5h1.25a3.25 3.25 0 0 1 0 6.5H23.5"
        fill="none"
        stroke="#FAF6EF"
        strokeWidth="2"
        strokeLinecap="round"
      />
      <path d="M13.5 16h7M13.5 19.5h4.5" stroke="#2B1D14" strokeWidth="1.6" strokeLinecap="round" />
    </svg>
  );
}

export function Logo({ className }: { className?: string }) {
  return (
    <span className={cn("inline-flex items-center gap-2.5", className)}>
      <LogoMark className="size-8 shrink-0" />
      <span className="font-display text-xl font-semibold tracking-tight">Kopi</span>
    </span>
  );
}
