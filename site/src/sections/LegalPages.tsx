import type { ReactNode } from "react";
import { ANALYTICS_SENTENCE, config, legalUpdated, NETWORK_STATEMENT } from "../config";
import { Footer, LegalNav } from "./Shell";

const BASE = import.meta.env.BASE_URL;

const Mail = () => <a href={`mailto:${config.contactEmail}`}>{config.contactEmail}</a>;

function Page({ title, children }: { title: string; children: ReactNode }) {
  return (
    <>
      <LegalNav />
      <main className="x-legal">
        <div className="x-wrap">
          <h1>{title}</h1>
          <p className="x-updated">Last updated: {legalUpdated}</p>
          {children}
        </div>
      </main>
      <Footer />
    </>
  );
}

function Refund() {
  return (
    <Page title="Refund policy">
      <p>Styky Pro is not on sale yet. This policy applies to the Supporter license from the day it goes on sale.</p>
      <h2>14 days, no questions</h2>
      <p>
        The Supporter license comes with a 14-day refund. Ask within 14 days of buying and you get your money back. We
        won&rsquo;t ask why.
      </p>
      <h2>How to ask</h2>
      <ul>
        <li>Reply to the Lemon Squeezy receipt email for your order, or</li>
        <li>
          email <Mail /> and tell us which purchase it was.
        </li>
      </ul>
      <h2>Who processes it</h2>
      <p>
        Lemon Squeezy is the merchant of record. It processes your payment, and it processes the refund too.
      </p>
      <h2>The free app</h2>
      <p>Styky itself is free and open source. A refund does not change that.</p>
    </Page>
  );
}

function Terms() {
  return (
    <Page title="Terms">
      <p>These terms cover this website and the Styky software, published by {config.publisher}.</p>
      <h2>Styky is free software</h2>
      <p>
        Styky is free, open-source software under the MIT license. It is provided as is, without warranty of any kind.
        Read the <a href={config.licenseUrl}>LICENSE</a> for the exact terms.
      </p>
      <h2>The Supporter license</h2>
      <p>
        The Supporter license is a one-time purchase. It unlocks Pro features for Styky 1.x on Windows. It is
        honor-based: the Pro code is in the public repository and there is no DRM. If you use Pro, please buy a
        license.
      </p>
      <h2>Buying</h2>
      <p>
        Supporter licenses are sold and processed by Lemon Squeezy, our merchant of record. Lemon
        Squeezy&rsquo;s <a href={config.lemonBuyerTermsUrl}>buyer terms</a> also apply to your purchase.
      </p>
      <h2>Refunds</h2>
      <p>
        Refunds follow the <a href={`${BASE}refund`}>refund policy</a>.
      </p>
      <h2>Contact</h2>
      <p>
        Questions: <Mail />.
      </p>
    </Page>
  );
}

function Privacy() {
  return (
    <Page title="Privacy">
      <p>{NETWORK_STATEMENT}</p>
      <p>{ANALYTICS_SENTENCE}</p>
      <h2>Your reminders</h2>
      <p>
        Your reminders and settings are stored locally, in a SQLite database on your PC. Nothing is sent to us. Styky
        has no accounts and no servers of its own.
      </p>
      <h2>When Styky goes online</h2>
      <ul>
        <li>
          <b>Updates.</b> When you click &ldquo;Check for updates&rdquo; in Settings, the direct-download version looks
          for a newer release on GitHub. The Microsoft Store version gets its updates from the Store.
        </li>
        <li>
          <b>Pro key.</b> When you click Activate or Remove, Styky sends your license key and a random activation name
          to Lemon Squeezy&rsquo;s license service.
        </li>
      </ul>
      <h2>Contact</h2>
      <p>
        Questions: <Mail />.
      </p>
    </Page>
  );
}

const PAGES = { refund: Refund, terms: Terms, privacy: Privacy } as const;
export type LegalPageName = keyof typeof PAGES;

export function LegalPage({ name }: { name: LegalPageName }) {
  const C = PAGES[name];
  return <C />;
}
