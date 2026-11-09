import { describe, expect, it } from "vitest";
import privacy from "../../docs/PRIVACY.md?raw";
import readme from "../../README.md?raw";
import siteConfig from "../../site/src/config.ts?raw";
import { NETWORK_STATEMENT } from "./network";

describe("the network statement", () => {
  it("is byte-identical in the app, docs/PRIVACY.md, the README and the site", () => {
    expect(NETWORK_STATEMENT).toBe(
      "No telemetry. Styky only goes online to check for updates and to activate or remove a Pro key when you click the button.",
    );
    const lines = (text: string) => text.split(/\r?\n/);
    expect(lines(privacy)).toContain(NETWORK_STATEMENT);
    expect(lines(readme)).toContain(`- ${NETWORK_STATEMENT}`);
    expect(siteConfig).toContain(`"${NETWORK_STATEMENT}"`);
  });
});
