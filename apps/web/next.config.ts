import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  /* config options here */
  cacheComponents: true,
  // 90 for the app screenshots, whose text blurs at the default 75.
  images: { qualities: [75, 90] },
  partialPrefetching: true,
  // The one-line install: curl -fsSL https://kopi.devanshuverma.in/install.sh | bash
  redirects() {
    return [
      {
        source: "/install.sh",
        destination: "https://raw.githubusercontent.com/code-devanshu/kopi/main/scripts/install.sh",
        permanent: false,
      },
    ];
  },
  turbopack: {
    rules: {
      "*.css": {
        loaders: ["@tailwindcss/turbopack"],
        as: "*.css",
      },
    },
  },
};

export default nextConfig;
