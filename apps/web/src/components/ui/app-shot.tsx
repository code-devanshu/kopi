import Image, { type StaticImageData } from "next/image";
import type { ReactNode } from "react";

import { cn } from "@/lib/cn";

/**
 * A screenshot of the real app, in the page's theme. Both images load lazily, so only the one
 * that shows is fetched. The shots come from the desktop app's mock backend (`pnpm dev:ui`,
 * with `?shot`), at 2x.
 */
export function AppShot({
  light,
  dark,
  alt,
  sizes,
  className,
  important = false,
}: {
  light: StaticImageData;
  dark: StaticImageData;
  alt: string;
  sizes: string;
  className?: string;
  /** The hero's shot: fetched before the rest of the page's images. */
  important?: boolean;
}) {
  const shared = { sizes, quality: 90, fetchPriority: important ? "high" : undefined } as const;
  return (
    <>
      <Image src={light} alt={alt} {...shared} className={cn("h-auto w-full dark:hidden", className)} />
      <Image src={dark} alt={alt} {...shared} className={cn("hidden h-auto w-full dark:block", className)} />
    </>
  );
}

/** A macOS window around a screenshot of one of Kopi's windows, such as Settings. */
export function WindowFrame({ title, children, className }: { title: string; children: ReactNode; className?: string }) {
  return (
    <div className={cn("overflow-hidden rounded-xl border border-border bg-surface shadow-lifted", className)}>
      <div className="relative flex h-9 items-center border-b border-border bg-surface-muted px-3.5" aria-hidden="true">
        <span className="flex gap-2">
          <span className="size-3 rounded-full bg-[#ff5f57]" />
          <span className="size-3 rounded-full bg-[#febc2e]" />
          <span className="size-3 rounded-full bg-[#28c840]" />
        </span>
        <span className="absolute inset-x-0 text-center text-xs font-medium text-muted-foreground">{title}</span>
      </div>
      {children}
    </div>
  );
}
