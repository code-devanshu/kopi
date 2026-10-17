import { cacheLife } from "next/cache";

import { siteConfig } from "@/lib/site";

export const platforms = ["macos", "windows"] as const;
export type Platform = (typeof platforms)[number];

export function isPlatform(value: string): value is Platform {
  return (platforms as readonly string[]).includes(value);
}

/**
 * The newest release's name, such as "0.1.0-alpha.6", from the file the app checks for updates.
 * GitHub's /releases/latest skips prereleases, and every alpha is one. The Updater workflow
 * updates the file only once a release is published, so it never names a draft.
 */
async function latestRelease(): Promise<string> {
  "use cache";
  // A new release reaches the buttons within minutes.
  cacheLife("minutes");
  const response = await fetch(`${siteConfig.links.github}/releases/download/updater/latest.json`);
  if (!response.ok) throw new Error(`latest.json: HTTP ${response.status}`);
  const { release } = (await response.json()) as { release?: unknown };
  if (typeof release !== "string" || !/^[\w.-]+$/.test(release)) {
    throw new Error("latest.json has no release name");
  }
  return release;
}

/** The newest release's file for the platform, as scripts/install.sh names it. */
export async function downloadUrl(platform: Platform): Promise<string> {
  const release = await latestRelease();
  const file = platform === "macos" ? `Kopi_${release}_universal.dmg` : `Kopi_${release}_x64-setup.exe`;
  return `${siteConfig.links.github}/releases/download/v${release}/${file}`;
}
