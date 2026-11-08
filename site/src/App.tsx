import { lazy, Suspense } from "react";
import { Footer, Hero, Nav } from "./sections/Chrome";
import { Faq, Features, KeyboardSection, Privacy, Problem } from "./sections/Content";
import { Pricing } from "./sections/Pricing";

const Grammar = lazy(() => import("./sections/Grammar").then((m) => ({ default: m.Grammar })));

export default function App() {
  return (
    <>
      <a className="x-skip" href="#why">
        Skip to content
      </a>
      <Nav />
      <main>
        <Hero />
        <Problem />
        <Features />
        <Suspense fallback={<section className="x-section alt" id="quick-add" style={{ minHeight: 640 }} />}>
          <Grammar />
        </Suspense>
        <KeyboardSection />
        <Privacy />
        <Pricing />
        <Faq />
      </main>
      <Footer />
    </>
  );
}
