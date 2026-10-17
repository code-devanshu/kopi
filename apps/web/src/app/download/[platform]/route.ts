import type { NextRequest } from "next/server";

import { downloadUrl, isPlatform } from "@/lib/downloads";
import { siteConfig } from "@/lib/site";

/**
 * /download/macos and /download/windows: the newest release's file. GitHub sends it as an
 * attachment, so the browser downloads it and stays on the page with the next steps.
 */
export async function GET(_request: NextRequest, ctx: RouteContext<"/download/[platform]">) {
  const { platform } = await ctx.params;
  if (!isPlatform(platform)) return new Response("Not found", { status: 404 });
  try {
    return Response.redirect(await downloadUrl(platform), 302);
  } catch (error) {
    // The releases page still has every file.
    console.error("kopi: couldn't find the newest release:", error);
    return Response.redirect(siteConfig.links.releases, 302);
  }
}
