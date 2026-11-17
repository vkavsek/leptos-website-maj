import { test, expect } from "@playwright/test";

test("shows survive client navigation and reload without browser clock errors", async ({ page }) => {
  const errors: string[] = [];
  page.on("pageerror", error => errors.push(error.message));
  await page.goto("/");
  // Wait for hydration, then navigate without a new server-rendered document.
  await expect.poll(() => page.locator("audio").evaluate(audio => (audio as HTMLAudioElement).volume)).toBe(0.5);
  await page.evaluate(() => { (window as any).showsNavigationMarker = true; });
  await page.getByRole("link", { name: "Shows", exact: true }).click();
  await page.locator(".shows-nav").getByRole("link", { name: "past", exact: true }).click();
  await expect(page).toHaveURL(/\/shows\/past$/);
  await expect(page.locator(".shows-list")).toBeVisible();
  expect(await page.locator(".show-container").count()).toBeGreaterThan(1);
  expect(await page.evaluate(() => (window as any).showsNavigationMarker)).toBe(true);

  await page.locator(".shows-nav").getByRole("link", { name: "future", exact: true }).click();
  await expect(page).toHaveURL(/\/shows\/future$/);
  await expect(page.locator("#shows-wrap")).toContainText(/Date:|There are currently no events/);
  await page.goto("/shows/past");
  await expect.poll(() => page.locator("audio").evaluate(audio => (audio as HTMLAudioElement).volume)).toBe(0.5);
  await expect(page.locator(".shows-list")).toBeVisible();
  expect(errors).toEqual([]);
});
