# Lemon Squeezy setup for Styky Pro

Owner checklist for selling **Styky Pro** (a one-time license key) through Lemon Squeezy. Once done, send the three values from
[step 6](#6-send-these-three-values) to the maintainer session. They unblock the v0.2.0 release.

**Why Lemon Squeezy:** it is the merchant of record, so it handles payments, global VAT/sales tax, receipts and refunds. It
generates license keys, and it officially supports **bank payouts to Bangladesh**
([supported countries](https://docs.lemonsqueezy.com/help/getting-started/supported-countries)).

> **Watch this:** Lemon Squeezy is moving its users toward *Stripe Managed Payments*
> ([their 2026 update](https://www.lemonsqueezy.com/blog/2026-update)), which does not support Bangladesh today. No shutdown
> date has been announced. If a forced migration ever excludes Bangladesh, the fallback is Paddle (see
> [If you have to leave Lemon Squeezy](#if-you-have-to-leave-lemon-squeezy)). The app keeps all licensing in one file
> (`src-tauri/src/license.rs`), so a switch is contained.

---

## 1. Create the account and store

The store belongs to the studio brand **Small Hinge Labs**, not to Styky. Styky Pro is the store's first product, and future apps
are added as more products in the same store. Each app's license check verifies its own product ID, so keys never cross between apps.

1. Sign up at <https://app.lemonsqueezy.com/register> with the brand address (e.g. `omor@smallhingelabs.com`).
2. Create a store:
   - **Store name:** `Small Hinge Labs`
   - **Store URL / subdomain:** `smallhingelabs` (this gives `smallhingelabs.lemonsqueezy.com`; the app reads the real link from you)
   - **Currency:** USD
3. **Store details** (Settings → General): website `https://smallhingelabs.com` (until that page exists,
   `https://styky.pages.dev`) and support `support@smallhingelabs.com`.

## 2. Get the store approved (activation)

New stores start in **test mode**. To take real payments, Lemon Squeezy reviews you. From the dashboard, click
**Activate store** and complete:

- **Identity verification:** your Bangladeshi passport or NID, plus a selfie. This is in your own name as an individual.
- **Business details:** individual / sole proprietor, country **Bangladesh**, your address.
- **Tax form:** as a non-US individual, fill in the **W-8BEN** they show you (it confirms you are not a US taxpayer). Your
  Bangladesh TIN goes there if you have one.
- **What you sell:** say something like *"Styky Pro: a one-time license key that unlocks premium features in Styky, an
  open-source Windows desktop reminder app. Digital software license, delivered instantly by email."* Link
  `https://styky.pages.dev`.

**What reviewers check** (all already in place):
- A live website that describes the product and its price ✅ (`styky.pages.dev` pricing section).
- A refund policy ✅ (the site's FAQ and pricing promise a refund; see [step 7](#7-refund-policy)).
- A privacy policy ✅ (`docs/PRIVACY.md`, linked in the site footer).
- A real product, not a placeholder ✅ (v0.1.0 is released and Pro v1 is built).

Approval usually takes a few days. If they ask questions, answer plainly. The open-source + paid-unlock model is common.

## 3. Payouts

Settings → **Payouts**.

- **Recommended: a Bangladeshi bank account in your name.** This is officially supported. Fill in account name (exactly as on the
  bank), account number, bank name, branch and **SWIFT/BIC** code (your bank provides it). Payouts are sent in USD and converted by
  the banks.
- **nsave US account (ACH):** Lemon Squeezy's docs **don't say** whether a Bangladesh-based seller may use a US bank account.
  **Ask support first** (chat bubble or `hello@lemonsqueezy.com`):
  > "I'm an individual seller based in Bangladesh. Can I set my payout destination to a US bank account (ACH) in my own name
  > held through nsave, or must it be a Bangladeshi bank account?"

  Only switch to nsave after a written "yes". A mismatched destination can hold payouts.
- **PayPal:** not an option. PayPal can't receive money in Bangladesh.

Payouts run on a schedule with a holding period (roughly twice a month after a ~2-week hold, per third-party reports). Check the
Payouts page for the current terms.

## 4. Create the product

Store → **Products → New product**.

| Field | Value |
|---|---|
| Name | `Styky Pro` |
| Description | Paste the block below |
| Pricing | **Single payment**, **$4.99** (early-bird; later raised to $6.99) |
| Tax category | **Software / SaaS** (digital, not physical) |
| Media | Upload `dist-store/box-art-2160x2160.png` (generate it with `node site/scripts/store-assets.mjs`) and `dist-store/screenshot-2-app.png` |
| Status | Published |

**Product description (paste):**

> Styky Pro is a one-time unlock for Styky, the always-visible reminder widget for Windows. Pay once, keep it forever, including
> future Pro features.
>
> - Connect Claude, ChatGPT and other AI assistants (MCP), plus a `styky` command-line tool: add, complete and snooze reminders
>   from chat or your terminal
> - Critical reminders that re-notify until done, and early alerts (5 minutes to 1 day before)
> - Advanced repeats: every 2 weeks, chosen weekdays, "last Friday of the month", end after N times
> - Themes: tint, transparency, density, the Sticky paper skin, font size
>
> Your license key arrives by email instantly. Paste it into Styky → Settings → Styky Pro. One key works on up to 3 of your PCs.
> Styky itself stays free and open source. Pro supports its development.

### License keys (important)

In the product editor open **License keys** and turn on **Generate license keys**:

| Setting | Value | Why |
|---|---|---|
| Activation limit | **3** | One key = 3 PCs. The app shows a clear message when the limit is reached, and **Remove license** in Settings frees a slot. |
| License length | **Unlimited / never expires** | Pro is a lifetime purchase. An expiring key would switch Pro off. |

### Confirmation email and receipt

In the product's **Confirmation modal / email** settings, add a short "next steps" text. The license key is included in the email
automatically.

> Thanks for supporting Styky! To unlock Pro: open Styky → Settings → **Styky Pro**, paste your license key, and click
> **Activate**. Need help? https://github.com/omorShahriar/styky/issues

Set the **button link** in the confirmation to `https://styky.pages.dev`.

## 5. Test a real purchase (no money)

Test-mode products get their own IDs, and the app only accepts keys from the **live** product. So test with the live product and a
100%-off code instead of test mode:

1. Store → **Discounts → New discount**: code `STYKYTEST`, **100%**, limited to **Styky Pro**, **max redemptions 5**.
2. Open the checkout link, apply `STYKYTEST`, complete the $0 order with your email.
3. Check that the email arrives with a license key.
4. Keep that key, because it's also useful for:
   - the maintainer's install test of v0.2.0
   - the Microsoft Store **Additional Testing Info → Credentials** table when the Pro version is submitted (Name: `Styky Pro
     license key`, Value: the key)
5. Afterwards, delete or expire the `STYKYTEST` code so it can't be shared.

## 6. Send these three values

| Value | Where to find it | Used for |
|---|---|---|
| **Store ID** (a number) | Settings → **Stores**: the ID next to your store, or the number in the dashboard URL | `LEMON_STORE_ID` in `src-tauri/src/license.rs` |
| **Product ID** (a number) | Products → Styky Pro: the number in the URL `…/products/<ID>` | `LEMON_PRODUCT_ID` in `src-tauri/src/license.rs` |
| **Checkout link** | Products → Styky Pro → **Share** → copy the checkout URL (`https://smallhingelabs.lemonsqueezy.com/buy/<uuid>`) | `STYKY_BUY_URL` in the app; `VITE_LS_CHECKOUT_URL` on the website |

The IDs are not secret (any key's validation response contains them). **Never send your Lemon Squeezy API key**: nothing in Styky
needs it.

**Until both IDs are filled in, the app accepts license keys from *any* Lemon Squeezy store**, so v0.2.0 is not released before
this step.

## 7. Refund policy

Lemon Squeezy, as merchant of record, processes refunds (Orders → the order → **Refund**). Refunding automatically disables the
license key, and Styky removes Pro on its next weekly check.

The website promises a **no-questions-asked refund**. Honor any request within 30 days; that is the industry norm and keeps
chargebacks (which cost fees) away.

## 8. What the maintainer does with the values (for reference)

1. Set `LEMON_STORE_ID`, `LEMON_PRODUCT_ID` and `STYKY_BUY_URL` in `src-tauri/src/license.rs`.
2. Cloudflare Pages → project `styky` → **Settings → Variables and secrets** → add `VITE_LS_CHECKOUT_URL` = the checkout link →
   redeploy. The website's Pro button switches from "Coming soon" to the early-bird buy button.
3. Update the website copy from "pre-order" to "available now", because Pro v1 ships in v0.2.0.
4. Build v0.2.0, test-install it with the `STYKYTEST` key, publish the GitHub release, and build the Store update package.
5. Microsoft Store (for the Pro version): tick *"purchases … not using the Microsoft Store commerce system"*, change the privacy
   answer to "Yes" with the PRIVACY.md link, update the age-rating purchases answer, and add the test key under Credentials.

## If you have to leave Lemon Squeezy

- **Paddle** (merchant of record for software; Bangladesh isn't on its unsupported list; pays by bank transfer). It reviews sellers
  first and has no built-in license keys, so Styky would need a small key service (a free Cloudflare Worker that issues and
  validates keys from Paddle webhooks).
- **What changes in the code:** only `license.rs` (the API calls) and the website's buy button. Existing Lemon Squeezy customers
  would get new keys by email, a one-time re-issue.
