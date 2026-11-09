import { proTeaser } from "../config";
import { Reveal } from "./util";

/** Pro teaser: four lines, no feature list, no Buy button, no email capture (the full /pro page ships later). */
export function Pricing() {
  return (
    <section className="x-section alt" id="pricing">
      <div className="x-wrap">
        <Reveal className="x-head center">
          <h2 className="x-h2">{proTeaser.heading}</h2>
          <p className="x-lead">{proTeaser.tagline}</p>
          <p className="x-teaser-price">{proTeaser.price}</p>
          <p className="x-lead">{proTeaser.line}</p>
        </Reveal>
      </div>
    </section>
  );
}
