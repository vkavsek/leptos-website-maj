import { test, expect } from "@playwright/test";

test("anonymous admin page is accessible and private without contacting Google", async ({ page }) => {
  const response = await page.goto("/admin");
  expect(response?.status()).toBe(401);
  expect(response?.headers()["cache-control"]).toBe("no-store");
  expect(response?.headers()["referrer-policy"]).toBe("no-referrer");
  await expect(page.getByRole("main")).toBeVisible();
  await expect(page.getByRole("heading", { level: 1 })).toHaveText("Administration sign-in");
  await page.keyboard.press("Tab");
  await expect(page.locator(":focus")).toHaveAttribute("href", /^(\/auth\/google\/start|\/)$/);
});

test("unbound callback and identity requests expose no identity or code", async ({ request }) => {
  const callback = await request.get("/auth/google/callback?state=invalid&code=fake-test-code");
  expect(callback.status()).toBe(403);
  expect(callback.headers()["cache-control"]).toBe("no-store");
  expect(await callback.text()).not.toContain("fake-test-code");
  const identity = await request.get("/auth/identity");
  expect(identity.status()).toBe(401);
  expect(identity.headers()["referrer-policy"]).toBe("no-referrer");
});

test("logout cannot mutate through GET and anonymous POST requires sign-in", async ({ request }) => {
  const read = await request.get("/auth/logout");
  expect(read.status()).toBe(405);
  expect(read.headers()["cache-control"]).toBe("no-store");
  const write = await request.post("/auth/logout");
  expect(write.status()).toBe(401);
  expect(write.headers()["cache-control"]).toBe("no-store");
});
