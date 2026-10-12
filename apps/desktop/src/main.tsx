import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { MainWindow } from "./windows/main/MainWindow";
import "./styles/global.css";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <MainWindow />
  </StrictMode>,
);
