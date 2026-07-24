import { test, expect, type Page } from "@playwright/test";
import { createMockWallet, type MockWallet } from "./fixtures/mockWallet";
import {
  createTestMint,
  fundAddress,
  mintTokensTo,
} from "./fixtures/testChain";

const CHAIN = process.env.VITE_CHAIN ?? "solana:devnet";

async function connectWallet(page: Page, wallet: MockWallet) {
  await page.goto("/");
  await page
    .getByRole("button", { name: new RegExp(`Connect ${wallet.name}`, "i") })
    .click();
  await expect(page.locator(".wallet-address")).toBeVisible();
}

test.describe.serial("subscription lifecycle", () => {
  const merchantWallet = createMockWallet(CHAIN, "Merchant Mock Wallet");
  const subscriberWallet = createMockWallet(CHAIN, "Subscriber Mock Wallet");
  let mint: Awaited<ReturnType<typeof createTestMint>>;

  test.beforeAll(async () => {
    mint = await createTestMint();
    await fundAddress(merchantWallet.address, 10);
    await fundAddress(subscriberWallet.address, 10);
    await mintTokensTo(mint.mint, mint.authority, merchantWallet.address, 0n);
    await mintTokensTo(
      mint.mint,
      mint.authority,
      subscriberWallet.address,
      1_000_000_000n,
    );
  });

  test.beforeEach(async ({ page }) => {
    await page.addInitScript(merchantWallet.initScript);
    await page.addInitScript(subscriberWallet.initScript);
  });

  test("7. shows an empty state before any plan exists", async ({ page }) => {
    await connectWallet(page, subscriberWallet);
    await page.getByRole("button", { name: "Browse plans" }).click();
    await expect(page.getByText("No active plans yet.")).toBeVisible();
  });

  test("1. merchant creates a variable-price plan", async ({ page }) => {
    await connectWallet(page, merchantWallet);
    await page.getByRole("button", { name: "Merchant" }).click();

    await page.getByLabel("Mint address").fill(mint.mint);
    await page.getByLabel("Amount per period").fill("1000");
    await page.getByLabel("Period (seconds)").fill("5");
    await page.getByLabel("Price mode").selectOption("1");
    await page.getByRole("button", { name: "Create plan" }).click();

    await expect(page.getByText(/1000 per 5s — active/)).toBeVisible({
      timeout: 15_000,
    });
  });

  test("2. subscriber subscribes to the plan", async ({ page }) => {
    await connectWallet(page, subscriberWallet);
    await page.getByRole("button", { name: "Browse plans" }).click();

    const card = page.locator(".plan-card").first();
    await expect(card).toBeVisible({ timeout: 15_000 });
    await card.getByLabel("Allowance").fill("100000");
    await card.getByLabel("Max amount per period").fill("1000");
    const subscribeButton = card.getByRole("button", { name: "Subscribe" });
    await subscribeButton.click();
    await expect(subscribeButton).toBeEnabled({ timeout: 15_000 });

    await page.getByRole("button", { name: "My subscriptions" }).click();
    await expect(page.getByText(/allowance remaining 100000/)).toBeVisible({
      timeout: 15_000,
    });
  });

  test("3. anyone can trigger a due charge", async ({ page }) => {
    await connectWallet(page, subscriberWallet);
    await page.getByRole("button", { name: "My subscriptions" }).click();

    const card = page.locator(".plan-card").first();
    await expect(card).toBeVisible({ timeout: 15_000 });
    await expect(card.getByText(/allowance remaining 100000/)).toBeVisible();
    await card.getByRole("button", { name: "Trigger charge" }).click();

    await expect(card.getByText(/allowance remaining 99000/)).toBeVisible({
      timeout: 15_000,
    });
  });

  test("4. merchant updates the variable price", async ({ page }) => {
    await connectWallet(page, merchantWallet);
    await page.getByRole("button", { name: "Merchant" }).click();

    const card = page.locator(".plan-card").first();
    await expect(card).toBeVisible({ timeout: 15_000 });
    await card.getByLabel(/New price/).fill("2000");
    await card.getByRole("button", { name: "Update price" }).click();

    await expect(card.getByText(/Pending price 2000 effective at/)).toBeVisible(
      { timeout: 15_000 },
    );
  });

  test("5. subscriber lowers their allowance", async ({ page }) => {
    await connectWallet(page, subscriberWallet);
    await page.getByRole("button", { name: "My subscriptions" }).click();

    const card = page.locator(".plan-card").first();
    await expect(card).toBeVisible({ timeout: 15_000 });
    await card.getByLabel("New allowance").fill("5000");
    await card.getByRole("button", { name: "Set allowance" }).click();

    await expect(card.getByText(/allowance remaining 5000/)).toBeVisible({
      timeout: 15_000,
    });
  });

  test("6. subscriber cancels", async ({ page }) => {
    await connectWallet(page, subscriberWallet);
    await page.getByRole("button", { name: "My subscriptions" }).click();

    const card = page.locator(".plan-card").first();
    await expect(card).toBeVisible({ timeout: 15_000 });
    await card.getByRole("button", { name: "Cancel" }).click();

    await expect(page.getByText("No subscriptions yet.")).toBeVisible({
      timeout: 15_000,
    });
  });
});
