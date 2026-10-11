// TODO: Replace the placeholder links below.
export const siteConfig = {
  name: "Kopi",
  tagline: "Everything you copy, on every computer.",
  description:
    "Kopi is an upcoming open-source clipboard manager for macOS and Windows. Unlimited history, instant search, pinboards and end-to-end encrypted sync. Coming soon.",
  url: process.env.NEXT_PUBLIC_SITE_URL ?? "https://kopi.devanshuverma.in",
  links: {
    github: "https://github.com/code-devanshu/kopi",
    twitter: "https://x.com/your-handle", // TODO: real X/Twitter profile
    contact: "mailto:code.devanshu@gmail.com", // TODO: real contact address
    privacy: "#privacy", // TODO: link to a privacy policy page once one exists
  },
} as const;

export const navLinks = [
  { href: "#features", label: "Features" },
  { href: "#privacy", label: "Privacy" },
  { href: "#open-source", label: "Open Source" },
  { href: "#faq", label: "FAQ" },
] as const;
