import React from "react";
import "./styles/globals.css";
import ReactDOM from "react-dom/client";
import App from "./app/Panel";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
