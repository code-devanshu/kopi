import type { ReactNode } from "react";

import { cn } from "@/lib/cn";

export const inputStyles =
  "w-full rounded-xl border border-border bg-background px-3.5 py-2.5 text-base text-foreground placeholder:text-muted-foreground/70 transition-colors focus:border-accent focus:outline-none focus-visible:outline-2 focus-visible:outline-offset-0 focus-visible:outline-accent aria-[invalid=true]:border-danger sm:text-sm";

export function FieldError({ id, message }: { id: string; message?: string }) {
  if (!message) return null;
  return (
    <p id={id} className="mt-1.5 text-sm text-danger">
      {message}
    </p>
  );
}

export function FieldLabel({ htmlFor, children, hint }: { htmlFor: string; children: ReactNode; hint?: string }) {
  return (
    <label htmlFor={htmlFor} className="mb-1.5 flex items-baseline justify-between text-sm font-medium">
      {children}
      {hint ? <span className="text-xs font-normal text-muted-foreground">{hint}</span> : null}
    </label>
  );
}

/** Pill-shaped radio/checkbox. The input stays in the DOM (visually hidden) for keyboard and screen readers. */
export function ChoiceChip({ children, className, ...input }: React.InputHTMLAttributes<HTMLInputElement> & { children: ReactNode }) {
  return (
    <label className={cn("relative cursor-pointer", className)}>
      <input {...input} className="peer sr-only" />
      <span className="flex h-10 items-center justify-center rounded-xl border border-border bg-background px-3 text-sm text-muted-foreground transition-colors peer-checked:border-accent peer-checked:bg-accent-soft peer-checked:font-medium peer-checked:text-foreground peer-focus-visible:outline-2 peer-focus-visible:outline-offset-2 peer-focus-visible:outline-accent hover:text-foreground">
        {children}
      </span>
    </label>
  );
}
