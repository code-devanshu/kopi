// URLs for the images Kopi serves itself, through the kopi:// protocol (protocol.rs). Both
// name the image by content hash.

// Windows webviews reach custom protocols over http, as Tauri's convertFileSrc does. In a
// browser, `pnpm dev:ui` has no kopi://, so the mock's sample images come from src/dev/images.
const BASE =
  import.meta.env.DEV && !("__TAURI_INTERNALS__" in window)
    ? "/src/dev/images/"
    : navigator.userAgent.includes("Windows")
      ? "http://kopi.localhost/"
      : "kopi://localhost/";

/** An image clip's 320 px thumbnail. Bump `attempt` to load it again once it's ready. */
export function thumbnailUrl(contentHash: string, attempt = 0): string {
  return `${BASE}thumb/${contentHash}${attempt ? `?attempt=${attempt}` : ""}`;
}

/** An image clip's full image. */
export function imageUrl(contentHash: string): string {
  return `${BASE}image/${contentHash}`;
}
