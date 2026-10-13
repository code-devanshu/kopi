import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { Panel } from "./windows/panel/Panel";
import "./styles/global.css";

// `pnpm dev:ui` in a browser has no Tauri backend, so it gets a fake one with sample clips.
// Production builds leave this out.
if (import.meta.env.DEV && !("__TAURI_INTERNALS__" in window)) {
  await import("./dev/mockBackend");
}

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <Panel />
  </StrictMode>,
);
