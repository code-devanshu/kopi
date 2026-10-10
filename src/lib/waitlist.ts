import { z } from "zod";

import {
  intentOptions,
  platformOptions,
  waitlistSchema,
  type WaitlistResult,
} from "@/lib/waitlist-schema";

const WEB3FORMS_ENDPOINT = "https://api.web3forms.com/submit";

/**
 * Validates a waitlist sign-up and emails it to the site owner through Web3Forms.
 *
 * Runs in the browser: the free Web3Forms plan only accepts client-side requests.
 * The access key is public by design — it only lets someone send mail to the
 * inbox it was created for. Set it as NEXT_PUBLIC_WEB3FORMS_KEY.
 *
 * `captchaToken` comes from the hCaptcha widget. Web3Forms verifies it and,
 * with hCaptcha turned on for the form, rejects submissions without one.
 */
export async function joinWaitlist(
  input: unknown,
  { honeypot = "", captchaToken }: { honeypot?: string; captchaToken: string | null },
): Promise<WaitlistResult> {
  // Honeypot: real people never see the hidden field. Pretend success for bots.
  if (honeypot.trim() !== "") {
    if (process.env.NODE_ENV === "development") {
      console.info("[waitlist] Honeypot field was filled; not sending", JSON.stringify(honeypot));
    }
    return { ok: true };
  }

  const parsed = waitlistSchema.safeParse(input);
  if (!parsed.success) {
    return {
      ok: false,
      message: "Please check the highlighted fields.",
      fieldErrors: z.flattenError(parsed.error).fieldErrors,
    };
  }

  if (!captchaToken) {
    return { ok: false, message: "Please confirm you're not a robot." };
  }

  const accessKey = process.env.NEXT_PUBLIC_WEB3FORMS_KEY;
  if (!accessKey) {
    if (process.env.NODE_ENV === "development") {
      console.info("[waitlist] NEXT_PUBLIC_WEB3FORMS_KEY is not set; not sending", parsed.data);
      return { ok: true };
    }
    console.error("[waitlist] NEXT_PUBLIC_WEB3FORMS_KEY is not set");
    return { ok: false, message: "The waitlist isn't open yet. Please try again later." };
  }

  const { name, email, platform, intents, message } = parsed.data;
  const label = <T extends { value: string; label: string }>(options: readonly T[], value: string) =>
    options.find((option) => option.value === value)?.label ?? value;

  try {
    const response = await fetch(WEB3FORMS_ENDPOINT, {
      method: "POST",
      headers: { "Content-Type": "application/json", Accept: "application/json" },
      body: JSON.stringify({
        access_key: accessKey,
        "h-captcha-response": captchaToken,
        subject: `Kopi waitlist: ${name || email}`,
        from_name: "Kopi waitlist",
        replyto: email,
        Name: name || "(not given)",
        Email: email,
        Platform: label(platformOptions, platform),
        "Wants to": intents.map((intent) => label(intentOptions, intent)).join(", "),
        Message: message || "(none)",
      }),
    });
    const result: { success?: boolean; message?: string } = await response.json().catch(() => ({}));
    if (!response.ok || !result.success) {
      throw new Error(result.message ?? `HTTP ${response.status}`);
    }
    if (process.env.NODE_ENV === "development") {
      console.info("[waitlist] Web3Forms response", response.status, result);
    }
    return { ok: true };
  } catch (error) {
    console.error("[waitlist] Failed to send submission", error);
    return {
      ok: false,
      message: "Something went wrong on our side. Please try again in a moment.",
    };
  }
}
