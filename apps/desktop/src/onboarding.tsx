import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { Onboarding } from "./windows/onboarding/Onboarding";
import "./styles/global.css";

// As in main.tsx: `pnpm dev:ui` in a browser gets a fake backend.
if (import.meta.env.DEV && !("__TAURI_INTERNALS__" in window)) {
  await import("./dev/mockBackend");
}

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <Onboarding />
  </StrictMode>,
);
