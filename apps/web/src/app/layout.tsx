import type { Metadata, Viewport } from "next";
import { Geist, Geist_Mono, Inter_Tight } from "next/font/google";

import { ClickTracker } from "@/components/analytics/click-tracker";
import { GoogleAnalytics } from "@/components/analytics/google-analytics";
import { MotionProvider } from "@/components/motion-provider";
import { ThemeScript } from "@/components/theme/theme-script";
import { siteConfig } from "@/lib/site";
import "./globals.css";

const heading = Inter_Tight({
  variable: "--font-heading",
  subsets: ["latin"],
});

const body = Geist({
  variable: "--font-body",
  subsets: ["latin"],
});

const code = Geist_Mono({
  variable: "--font-code",
  subsets: ["latin"],
});

const title = `${siteConfig.name}: the open-source clipboard manager for Mac and Windows`;

export const metadata: Metadata = {
  metadataBase: new URL(siteConfig.url),
  title: {
    default: title,
    template: `%s · ${siteConfig.name}`,
  },
  description: siteConfig.description,
  applicationName: siteConfig.name,
  keywords: ["clipboard manager", "clipboard history", "macOS", "Windows", "open source", "sync", "privacy"],
  alternates: { canonical: "/" },
  openGraph: {
    type: "website",
    url: "/",
    siteName: siteConfig.name,
    title,
    description: siteConfig.description,
    locale: "en_US",
  },
  twitter: {
    card: "summary_large_image",
    title,
    description: siteConfig.description,
  },
};

export const viewport: Viewport = {
  themeColor: [
    { media: "(prefers-color-scheme: light)", color: "#faf6ef" },
    { media: "(prefers-color-scheme: dark)", color: "#16100c" },
  ],
};

export default function RootLayout({ children }: LayoutProps<"/">) {
  return (
    <html
      lang="en"
      suppressHydrationWarning
      className={`${heading.variable} ${body.variable} ${code.variable} antialiased`}
    >
      <head>
        <ThemeScript />
      </head>
      <body className="min-h-dvh font-sans">
        <a
          href="#main"
          className="sr-only focus:not-sr-only focus:fixed focus:top-3 focus:left-3 focus:z-60 focus:rounded-full focus:bg-accent focus:px-4 focus:py-2 focus:text-accent-foreground"
        >
          Skip to content
        </a>
        <MotionProvider>{children}</MotionProvider>
        <ClickTracker />
      </body>
      <GoogleAnalytics />
    </html>
  );
}
