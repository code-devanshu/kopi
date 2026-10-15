// TODO: Replace the placeholder links below.
export const siteConfig = {
  name: "Kopi",
  tagline: "Everything you copy, on every computer.",
  description:
    "Kopi is an open-source clipboard manager for macOS and Windows. Unlimited history, instant search and pinboards, kept on your computer. The alpha is free to download.",
  url: process.env.NEXT_PUBLIC_SITE_URL ?? "https://kopi.devanshuverma.in",
  // /install.sh redirects to scripts/install.sh on main, in next.config.ts.
  installCommand: "curl -fsSL https://kopi.devanshuverma.in/install.sh | bash",
  links: {
    github: "https://github.com/code-devanshu/kopi",
    // The releases page, not a file, so it doesn't change with every alpha.
    releases: "https://github.com/code-devanshu/kopi/releases",
    install: "https://github.com/code-devanshu/kopi/blob/main/docs/INSTALL_ALPHA.md",
    installWindows: "https://github.com/code-devanshu/kopi/blob/main/docs/INSTALL_ALPHA.md#windows",
    license: "https://github.com/code-devanshu/kopi/blob/main/LICENSE",
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
