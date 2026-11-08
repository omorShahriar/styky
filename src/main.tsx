import React from "react";
import "./styles/globals.css";
import ReactDOM from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import Panel from "./app/Panel";
import QuickAdd from "./app/QuickAdd";
import { MotionConfig } from "motion/react";

// One bundle, one page; the window label picks which app to show.
const label = getCurrentWindow().label;
const App = label === "quickadd" ? QuickAdd : Panel;

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <MotionConfig reducedMotion="user">
      <App />
    </MotionConfig>
  </React.StrictMode>,
);
