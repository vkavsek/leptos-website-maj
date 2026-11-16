import { test, expect } from "@playwright/test";

// Only the isolated Rust test harness supplies a pre-approved session. No login bypass exists.
test.skip(!process.env.MAJ_TEST_ADMIN_COOKIES, "Run with just test-admin-browser");

test("native content forms publish and remove live records", async ({ page, context, request, baseURL }, testInfo) => {
  const cookie: string = JSON.parse(process.env.MAJ_TEST_ADMIN_COOKIES!)[testInfo.project.name];
  const split = cookie.indexOf("=");
  await context.addCookies([{ name: cookie.slice(0, split), value: cookie.slice(split + 1), url: baseURL! }]);
  await page.setViewportSize({ width: 375, height: 812 });
  await page.goto("/admin");
  await expect(page.getByRole("heading", { name: "Administration", exact: true })).toBeVisible();
  const showName = `Browser ${testInfo.project.name} <script>literal</script>`;
  const show = page.getByRole("form", { name: "Shows", exact: true });
  await show.getByLabel("Date (required)").fill("2028-02-29");
  await show.getByLabel("Event name (optional)").fill(showName);
  const submitting = page.waitForRequest(request => request.method() === "POST" && request.url().endsWith("/admin/content"));
  await show.getByRole("button", { name: "Add show" }).click();
  expect((await submitting).headers()["origin"]).toBe(new URL(baseURL!).origin);
  await expect(page.locator("#shows")).toContainText(showName);
  expect(await (await request.get("/api/get_shows")).text()).toContain(showName);

  const videoId = testInfo.project.name === "chromium" ? "3uh_Jc1prRQ" : "dQw4w9WgXcQ";
  await page.getByLabel("YouTube URL (required)").fill(`https://youtu.be/${videoId}`);
  await page.getByRole("button", { name: "Add video" }).click();
  await expect(page.locator("#videos")).toContainText(videoId);
  expect(await (await request.get("/api/get_media_videos")).text()).toContain(`/embed/${videoId}`);

  const photoName = `Browser ${testInfo.project.name}.PNG`;
  const normalizedName = `browser_${testInfo.project.name}.png`;
  const photo = await page.screenshot();
  await page.getByLabel("Photo (required)").setInputFiles({ name: photoName, mimeType: "image/png", buffer: photo });
  await page.getByRole("button", { name: "Add photo" }).click();
  await expect(page.locator("#gallery")).toContainText(normalizedName);
  const imageUrl = `/img/about_pics/${normalizedName}`;
  expect((await request.get(imageUrl)).status()).toBe(200);
  expect((await request.get(`/img/about_thumbnails/${normalizedName}.webp`)).status()).toBe(200);
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= document.documentElement.clientWidth)).toBe(true);

  for (const [kind, label] of [["shows", showName], ["videos", videoId], ["gallery", normalizedName]]) {
    const form = page.locator(`#${kind} li`).filter({ hasText: label }).getByRole("form");
    const confirmation = form.getByRole("checkbox", { name: "Confirm permanent removal" });
    await expect(confirmation).not.toBeChecked();
    await expect(confirmation).toHaveAttribute("required", "");
    await confirmation.focus();
    await page.keyboard.press("Space");
    await form.getByRole("button", { name: "Remove", exact: true }).click();
    await expect(form).toHaveCount(0);
  }
  expect((await request.get(imageUrl)).status()).toBe(404);
  expect(await (await request.get("/api/get_shows")).text()).not.toContain(showName);
  expect(await (await request.get("/api/get_media_videos")).text()).not.toContain(videoId);
  await page.getByRole("button", { name: "Sign out", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Administration sign-in", exact: true })).toBeVisible();
});
