import { z } from "zod";

export const platformOptions = [
  { value: "macos", label: "macOS" },
  { value: "windows", label: "Windows" },
  { value: "both", label: "Both" },
] as const;

export const intentOptions = [
  { value: "use", label: "Use it" },
  { value: "contribute", label: "Contribute code" },
  { value: "test", label: "Help test" },
  { value: "design", label: "Design" },
] as const;

const platformValues = ["macos", "windows", "both"] as const;
const intentValues = ["use", "contribute", "test", "design"] as const;

/** Shared by the form (react-hook-form) and the submit in src/lib/waitlist.ts. */
export const waitlistSchema = z.object({
  name: z.string().trim().max(80, { error: "Keep it under 80 characters." }),
  email: z
    .string()
    .trim()
    .min(1, { error: "We need an email to reach you." })
    .pipe(z.email({ error: "That doesn't look like an email address." })),
  platform: z.enum(platformValues, { error: "Pick the platform you use." }),
  intents: z
    .array(z.enum(intentValues))
    .min(1, { error: "Choose at least one option." }),
  message: z.string().trim().max(1000, { error: "Keep it under 1000 characters." }),
});

export type WaitlistInput = z.input<typeof waitlistSchema>;
export type WaitlistSubmission = z.output<typeof waitlistSchema>;

export type WaitlistResult =
  | { ok: true }
  | {
      ok: false;
      message: string;
      fieldErrors?: Partial<Record<keyof WaitlistSubmission, string[]>>;
    };
