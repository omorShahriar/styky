import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "./styles/site.css";
import { LegalPage, type LegalPageName } from "./sections/LegalPages";

/** Entry for refund.html, terms.html and privacy.html: the page is named by <body data-page="...">. */
const name = document.body.dataset.page as LegalPageName;

createRoot(document.getElementById("root") as HTMLElement).render(
  <StrictMode>
    <LegalPage name={name} />
  </StrictMode>,
);
