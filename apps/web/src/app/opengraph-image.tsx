import { ImageResponse } from "next/og";

export const alt = "Kopi: everything you copy, on every computer. Open-source clipboard manager, in alpha on Mac and Windows.";
export const size = { width: 1200, height: 630 };
export const contentType = "image/png";

export default function OpengraphImage() {
  return new ImageResponse(
    (
      <div
        style={{
          width: "100%",
          height: "100%",
          display: "flex",
          flexDirection: "column",
          justifyContent: "space-between",
          padding: 80,
          background: "radial-gradient(circle at 80% 20%, #f6e4cc 0%, #faf6ef 55%)",
          color: "#2b1d14",
        }}
      >
        <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
          <div style={{ display: "flex", alignItems: "center", gap: 20 }}>
            <svg width="72" height="72" viewBox="0 0 32 32">
              <rect width="32" height="32" rx="8" fill="#2B1D14" />
              <rect x="6.5" y="6" width="13" height="14" rx="3" fill="#D08A3A" />
              <rect x="10.5" y="11" width="13" height="15" rx="3" fill="#FAF6EF" />
              <path d="M23.5 14.5h1.25a3.25 3.25 0 0 1 0 6.5H23.5" fill="none" stroke="#FAF6EF" strokeWidth="2" strokeLinecap="round" />
              <path d="M13.5 16h7M13.5 19.5h4.5" stroke="#2B1D14" strokeWidth="1.6" strokeLinecap="round" />
            </svg>
            <div style={{ fontSize: 48, fontWeight: 700 }}>Kopi</div>
          </div>
          <div
            style={{
              display: "flex",
              padding: "10px 22px",
              borderRadius: 999,
              background: "#f6e4cc",
              color: "#93561a",
              fontSize: 24,
            }}
          >
            Alpha for Mac and Windows
          </div>
        </div>
        <div style={{ display: "flex", flexDirection: "column", gap: 24 }}>
          <div style={{ fontSize: 76, fontWeight: 700, letterSpacing: -2, lineHeight: 1.05, maxWidth: 900 }}>
            Everything you copy, on every computer.
          </div>
          <div style={{ fontSize: 28, color: "#6b5444" }}>
            Open-source clipboard manager for macOS and Windows
          </div>
        </div>
      </div>
    ),
    size,
  );
}
