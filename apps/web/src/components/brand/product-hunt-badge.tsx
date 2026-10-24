import { cn } from "@/lib/cn";

/**
 * Launch day: the launch badge sits in the homepage hero and at the top of /download, with a
 * line saying Kopi is live. Set to false afterwards and it moves to the footer, next to the
 * follow badge, as a lasting "Featured on Product Hunt".
 */
export const SHOW_PH_LAUNCH_BADGE_IN_HERO = true;

const alt = "Kopi - Open-source clipboard manager for Mac and Windows | Product Hunt";

// Product Hunt's embed codes, as they gave them. Their links and UTM parameters stay exact,
// even the follow and review links naming utm_source twice.
const badges = {
  launch: {
    href: "https://www.producthunt.com/products/kopi-2?embed=true&utm_source=badge-featured&utm_medium=badge&utm_campaign=badge-kopi-2",
    light: "https://api.producthunt.com/widgets/embed-image/v1/featured.svg?post_id=1290235&theme=light&t=1792835471369",
    dark: "https://api.producthunt.com/widgets/embed-image/v1/featured.svg?post_id=1290235&theme=dark&t=1792835471369",
    width: 250,
    height: 54,
  },
  follow: {
    href: "https://www.producthunt.com/products/kopi-2?utm_source=badge-follow&utm_medium=badge&utm_source=badge-kopi-2",
    light: "https://api.producthunt.com/widgets/embed-image/v1/follow.svg?product_id=1353893&theme=light&size=small",
    dark: "https://api.producthunt.com/widgets/embed-image/v1/follow.svg?product_id=1353893&theme=dark&size=small",
    width: 86,
    height: 32,
  },
  review: {
    href: "https://www.producthunt.com/products/kopi-2/reviews/new?utm_source=badge-product_review&utm_medium=badge&utm_source=badge-kopi-2",
    light: "https://api.producthunt.com/widgets/embed-image/v1/product_review.svg?product_id=1353893&theme=light",
    dark: "https://api.producthunt.com/widgets/embed-image/v1/product_review.svg?product_id=1353893&theme=dark",
    width: 250,
    height: 54,
  },
} as const;

export type ProductHuntBadgeVariant = keyof typeof badges;

/**
 * A Product Hunt badge. The site's theme toggle can differ from the system's, so both images
 * render and CSS shows the one for the site's theme, not a prefers-color-scheme <picture>.
 * A lazy image that's hidden isn't fetched, so lazy badges only load the theme in use.
 */
export function ProductHuntBadge({
  variant,
  location,
  lazy = true,
  className,
}: {
  variant: ProductHuntBadgeVariant;
  location: string;
  lazy?: boolean;
  className?: string;
}) {
  const badge = badges[variant];
  const image = { width: badge.width, height: badge.height, loading: lazy ? "lazy" : "eager" } as const;

  return (
    <a
      href={badge.href}
      target="_blank"
      rel="noopener noreferrer"
      data-track="product_hunt_click"
      data-track-badge={variant}
      data-track-location={location}
      className={cn("inline-flex shrink-0 rounded-lg", className)}
    >
      {/* eslint-disable-next-line @next/next/no-img-element */}
      <img src={badge.light} alt={alt} {...image} className="dark:hidden" />
      {/* eslint-disable-next-line @next/next/no-img-element */}
      <img src={badge.dark} alt={alt} {...image} className="hidden dark:block" />
    </a>
  );
}

/** The launch-day line and badge for the top of a page. Nothing once the launch is over. */
export function ProductHuntLaunch({ location, className }: { location: string; className?: string }) {
  if (!SHOW_PH_LAUNCH_BADGE_IN_HERO) return null;
  return (
    <div className={cn("flex flex-col items-center justify-center gap-3 sm:flex-row sm:gap-4", className)}>
      <p className="text-sm font-medium text-muted-foreground">We&apos;re live on Product Hunt today</p>
      <ProductHuntBadge variant="launch" location={location} lazy={false} />
    </div>
  );
}
