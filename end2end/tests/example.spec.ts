import { test, expect } from "@playwright/test";

test("homepage has the musician's title and navigation", async ({ page }) => {
  await page.goto("/");

  await expect(page).toHaveTitle("Home - Maj Kavšek");

  await expect(page.getByRole("heading", { level: 1 })).toHaveText("Maj Kavšek");
  await expect(page.getByRole("link", { name: "About Me", exact: true })).toHaveAttribute("href", "/about_me");
});
