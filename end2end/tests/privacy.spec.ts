import { test, expect } from "@playwright/test";

// Legal links are discoverable before Google login and without hydration.
test.use({ javaScriptEnabled: false });

test.beforeEach(async ({ context, baseURL }) => {
  const origin = new URL(baseURL!).origin;
  await context.route("**/*", route =>
    new URL(route.request().url()).origin === origin ? route.continue() : route.abort()
  );
});

test("admin sign-in links to public information and privacy without changing Home", async ({ page, context }) => {
  await page.goto("/");
  await expect(page.locator(".home-intro")).toHaveCount(0);
  await expect(page.getByRole("link", { name: "Site information", exact: true })).toHaveCount(0);
  await expect(page.getByRole("link", { name: "Privacy policy", exact: true })).toHaveCount(0);
  const admin = await page.goto("/admin");
  expect(admin?.status()).toBe(401);
  await expect(page.getByRole("link", { name: "Privacy policy", exact: true })).toHaveAttribute("href", "/privacy");
  await page.getByRole("link", { name: "Site information", exact: true }).click();
  await expect(page).toHaveURL(/\/info$/);
  await expect(page).toHaveTitle("Site information - Maj Kavšek");
  await expect(page.locator(".home-intro")).toContainText("Maj Kavšek’s Portfolio");
  const link = page.getByRole("link", { name: "Privacy policy", exact: true });
  await expect(link).toHaveAttribute("href", "/privacy");
  await link.click();
  await expect(page).toHaveURL(/\/privacy$/);
  await expect(page).toHaveTitle("Privacy policy - Maj Kavšek");
  await expect(page.getByRole("heading", { level: 1 })).toHaveText("Privacy policy");
  expect((await context.cookies()).some(cookie => cookie.name.includes("maj_admin"))).toBe(false);
});

for (const viewport of [{ width: 1920, height: 1080 }, { width: 390, height: 700 }]) {
  test(`public documents fit and privacy is keyboard-scrollable at ${viewport.width}×${viewport.height}`, async ({ page }) => {
    await page.setViewportSize(viewport);
    await page.goto("/admin");
    await page.getByRole("link", { name: "Site information", exact: true }).click();
    await expect(page).toHaveURL(/\/info$/);
    const infoResponse = await page.goto("/info");
    expect(infoResponse?.status()).toBe(200);
    const info = page.getByRole("article", { name: "Site information", exact: true });
    await expect(info.getByRole("link", { name: "Privacy policy", exact: true })).toBeInViewport();
    expect(await info.evaluate(element => element.scrollWidth - element.clientWidth)).toBeLessThanOrEqual(1);
    const response = await page.goto("/privacy");
    expect(response?.status()).toBe(200);
    const policy = page.getByRole("article", { name: "Privacy policy", exact: true });
    await expect(policy).toContainText("does not request your Google email");
    await expect(policy).toContainText("Disabling an administrator invalidates access but keeps the identity");
    await expect(policy).toContainText("info{at}majkavsek.com");
    await expect(policy).toContainText("has not configured fixed deletion periods");
    expect(await policy.evaluate(element => element.scrollWidth - element.clientWidth)).toBeLessThanOrEqual(1);
    const bounds = (await policy.boundingBox())!;
    expect(bounds.x).toBeGreaterThanOrEqual(0);
    expect(bounds.x + bounds.width).toBeLessThanOrEqual(viewport.width);
    expect(bounds.y).toBeGreaterThanOrEqual(0);
    expect(bounds.y + bounds.height).toBeLessThanOrEqual(viewport.height);
    await policy.focus();
    await page.keyboard.press("End");
    await expect.poll(() => policy.evaluate(element => element.scrollTop)).toBeGreaterThan(0);
    await expect(page.getByRole("link", { name: "Back to homepage", exact: true })).toBeInViewport();
  });
}

test("anonymous public browsing does not set cookies or advertise administration", async ({ page, context }) => {
  const cookies: string[] = [];
  page.on("response", response => {
    const value = response.headers()["set-cookie"];
    if (value) cookies.push(value);
  });
  for (const path of ["/", "/about_me", "/shows", "/media", "/discography", "/info", "/privacy"]) {
    const response = await page.goto(path);
    expect(response?.status()).toBe(200);
    await expect(page.locator('a[href="/admin"], a[href^="/auth/"]')).toHaveCount(0);
  }
  expect(cookies).toEqual([]);
  expect(await context.cookies()).toEqual([]);
});
