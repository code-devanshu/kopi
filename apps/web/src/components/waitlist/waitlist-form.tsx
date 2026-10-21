"use client";

import { zodResolver } from "@hookform/resolvers/zod";
import { AnimatePresence, motion } from "motion/react";
import { CircleAlert, LoaderCircle } from "lucide-react";
import { useRef, useState, type BaseSyntheticEvent } from "react";
import { useForm } from "react-hook-form";

import { buttonStyles } from "@/components/ui/button-styles";
import { ChoiceChip, FieldError, FieldLabel, inputStyles } from "@/components/waitlist/form-fields";
import { SuccessMessage } from "@/components/waitlist/success-message";
import { WaitlistCaptcha } from "@/components/waitlist/waitlist-captcha";
import {
  intentOptions,
  platformOptions,
  waitlistSchema,
  type WaitlistInput,
  type WaitlistSubmission,
} from "@/lib/waitlist-schema";
import { trackEvent } from "@/lib/analytics";
import { joinWaitlist } from "@/lib/waitlist";

const defaultValues: Partial<WaitlistInput> = {
  name: "",
  email: "",
  intents: ["use"],
  message: "",
};

export function WaitlistForm() {
  const [submitted, setSubmitted] = useState(false);
  const [submitError, setSubmitError] = useState<string | null>(null);
  // Changing the key remounts the widget, which gives a fresh captcha.
  const [captchaKey, setCaptchaKey] = useState(0);
  const [captchaToken, setCaptchaToken] = useState<string | null>(null);
  const [captchaError, setCaptchaError] = useState<string | null>(null);
  const started = useRef(false);

  const {
    register,
    handleSubmit,
    reset,
    setError,
    formState: { errors, isSubmitting },
  } = useForm<WaitlistInput, unknown, WaitlistSubmission>({
    resolver: zodResolver(waitlistSchema),
    defaultValues,
  });

  function onCaptchaToken(token: string | null) {
    setCaptchaToken(token);
    if (token) setCaptchaError(null);
  }

  function checkCaptcha() {
    if (captchaToken) return true;
    setCaptchaError("Please confirm you're not a robot.");
    return false;
  }

  function onStart() {
    if (started.current) return;
    started.current = true;
    trackEvent("waitlist_form_start");
  }

  function onInvalid() {
    checkCaptcha();
    trackEvent("waitlist_signup_error", { reason: "invalid_fields" });
  }

  async function onSubmit(data: WaitlistSubmission, event?: BaseSyntheticEvent) {
    setSubmitError(null);
    if (!checkCaptcha()) {
      trackEvent("waitlist_signup_error", { reason: "captcha_missing" });
      return;
    }
    const form = event?.target instanceof HTMLFormElement ? event.target : null;
    const honeypot = form ? String(new FormData(form).get("kopi_hp") ?? "") : "";
    try {
      const result = await joinWaitlist(data, { honeypot, captchaToken });
      // A token can only be used once, so ask for a fresh one before any retry.
      setCaptchaKey((key) => key + 1);
      setCaptchaToken(null);
      if (result.ok) {
        // A filled honeypot means a bot; it gets a fake success but isn't counted.
        if (!honeypot) {
          trackEvent("waitlist_signup", { platform: data.platform, intents: data.intents.join(",") });
        }
        setSubmitted(true);
        reset(defaultValues);
        return;
      }
      setSubmitError(result.message);
      trackEvent("waitlist_signup_error", { reason: result.fieldErrors ? "invalid_fields" : "send_failed" });
      for (const [field, messages] of Object.entries(result.fieldErrors ?? {})) {
        if (messages?.[0]) setError(field as keyof WaitlistSubmission, { message: messages[0] });
      }
    } catch {
      trackEvent("waitlist_signup_error", { reason: "network" });
      setSubmitError("We couldn't send your sign-up. Check your connection and try again.");
    }
  }

  return (
    <div className="rounded-3xl border border-border bg-surface p-6 shadow-lifted sm:p-8">
      <AnimatePresence mode="wait" initial={false}>
        {submitted ? (
          <SuccessMessage key="success" onReset={() => setSubmitted(false)} />
        ) : (
          <motion.form
            key="form"
            noValidate
            onSubmit={handleSubmit(onSubmit, onInvalid)}
            onFocus={onStart}
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0, scale: 0.98 }}
            className="space-y-5"
          >
            <div className="grid gap-5 sm:grid-cols-2">
              <div>
                <FieldLabel htmlFor="wl-name" hint="Optional">
                  Name
                </FieldLabel>
                <input
                  id="wl-name"
                  type="text"
                  autoComplete="name"
                  placeholder="Ada Lovelace"
                  aria-invalid={errors.name ? true : undefined}
                  aria-describedby={errors.name ? "wl-name-error" : undefined}
                  className={inputStyles}
                  {...register("name")}
                />
                <FieldError id="wl-name-error" message={errors.name?.message} />
              </div>
              <div>
                <FieldLabel htmlFor="wl-email" hint="Required">
                  Email
                </FieldLabel>
                <input
                  id="wl-email"
                  type="email"
                  inputMode="email"
                  autoComplete="email"
                  placeholder="you@example.com"
                  aria-required="true"
                  aria-invalid={errors.email ? true : undefined}
                  aria-describedby={errors.email ? "wl-email-error" : undefined}
                  className={inputStyles}
                  {...register("email")}
                />
                <FieldError id="wl-email-error" message={errors.email?.message} />
              </div>
            </div>

            <fieldset aria-describedby={errors.platform ? "wl-platform-error" : undefined}>
              <legend className="mb-1.5 text-sm font-medium">Which platform do you use?</legend>
              <div className="grid grid-cols-3 gap-2">
                {platformOptions.map((option) => (
                  <ChoiceChip key={option.value} type="radio" value={option.value} {...register("platform")}>
                    {option.label}
                  </ChoiceChip>
                ))}
              </div>
              <FieldError id="wl-platform-error" message={errors.platform?.message} />
            </fieldset>

            <fieldset aria-describedby={errors.intents ? "wl-intents-error" : undefined}>
              <legend className="mb-1.5 text-sm font-medium">
                I want to <span className="font-normal text-muted-foreground">(pick any)</span>
              </legend>
              <div className="grid grid-cols-2 gap-2">
                {intentOptions.map((option) => (
                  <ChoiceChip key={option.value} type="checkbox" value={option.value} {...register("intents")}>
                    {option.label}
                  </ChoiceChip>
                ))}
              </div>
              <FieldError id="wl-intents-error" message={errors.intents?.message} />
            </fieldset>

            <div>
              <FieldLabel htmlFor="wl-message" hint="Optional">
                Anything we should know?
              </FieldLabel>
              <textarea
                id="wl-message"
                rows={3}
                placeholder="Your setup, a feature you'd love, how you'd like to help…"
                aria-invalid={errors.message ? true : undefined}
                aria-describedby={errors.message ? "wl-message-error" : undefined}
                className={`${inputStyles} resize-y`}
                {...register("message")}
              />
              <FieldError id="wl-message-error" message={errors.message?.message} />
            </div>

            <div>
              <WaitlistCaptcha key={captchaKey} onToken={onCaptchaToken} />
              <FieldError id="wl-captcha-error" message={captchaError ?? undefined} />
            </div>

            {/* Honeypot: hidden from people and assistive tech; bots tend to fill it in.
                Named and labelled so browser autofill and password managers leave it alone. */}
            <div aria-hidden="true" className="absolute left-[-9999px] h-px w-px overflow-hidden">
              <label htmlFor="wl-hp">Leave this field empty</label>
              <input
                id="wl-hp"
                name="kopi_hp"
                type="text"
                tabIndex={-1}
                autoComplete="off"
                data-1p-ignore
                data-lpignore="true"
                data-bwignore
              />
            </div>

            <AnimatePresence>
              {submitError ? (
                <motion.p
                  role="alert"
                  initial={{ opacity: 0, height: 0 }}
                  animate={{ opacity: 1, height: "auto" }}
                  exit={{ opacity: 0, height: 0 }}
                  className="flex items-start gap-2 overflow-hidden rounded-xl border border-danger/30 bg-danger/10 px-3.5 py-3 text-sm text-danger"
                >
                  <CircleAlert className="mt-0.5 size-4 shrink-0" aria-hidden="true" />
                  {submitError}
                </motion.p>
              ) : null}
            </AnimatePresence>

            <button type="submit" disabled={isSubmitting} className={buttonStyles({ size: "lg", className: "w-full" })}>
              {isSubmitting ? (
                <>
                  <LoaderCircle className="size-4 animate-spin" aria-hidden="true" />
                  Joining…
                </>
              ) : (
                "Join the waitlist"
              )}
            </button>
            <p className="text-center text-xs text-muted-foreground">
              We&apos;ll only email you about Kopi. No spam, and you can opt out at any time.
            </p>
          </motion.form>
        )}
      </AnimatePresence>
    </div>
  );
}
