import React from "react";
import "./styles/globals.css";
import ReactDOM from "react-dom/client";
import App from "./app/Panel";
import { MotionConfig } from "motion/react";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <MotionConfig reducedMotion="user">
      <App />
    </MotionConfig>
  </React.StrictMode>,
);
