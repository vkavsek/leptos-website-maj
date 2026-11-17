import { test, expect } from "@playwright/test";

// Only the isolated Rust test harness supplies a pre-approved session. No login bypass exists.
test.skip(!process.env.MAJ_TEST_ADMIN_COOKIES, "Run with just test-admin-browser");

test("native content forms publish and remove live records", async ({ page, context, request, baseURL }, testInfo) => {
  const cookie: string = JSON.parse(process.env.MAJ_TEST_ADMIN_COOKIES!)[testInfo.project.name];
  const split = cookie.indexOf("=");
  await context.addCookies([{ name: cookie.slice(0, split), value: cookie.slice(split + 1), url: baseURL! }]);
  await page.goto("/admin");
  await expect(page.getByRole("link", { name: "Site information", exact: true })).toHaveAttribute("href", "/info");
  await expect(page.getByRole("link", { name: "Privacy policy", exact: true })).toHaveAttribute("href", "/privacy");
  await page.setViewportSize({ width: 375, height: 812 });
  await page.goto("/admin");
  await expect(page.getByRole("heading", { name: "Administration", exact: true })).toBeVisible();
  const navigation = page.getByRole("navigation", { name: "Content editors" });
  const select = async (kind: string) => {
    const labels: Record<string, string> = { gallery: "Gallery", shows: "Shows", videos: "YouTube videos" };
    const label = labels[kind];
    await navigation.getByRole("link", { name: label, exact: true }).click();
    await expect(page).toHaveURL(new RegExp(`section=${kind}$`));
    await expect(navigation.getByRole("link", { name: label, exact: true })).toHaveAttribute("aria-current", "page");
    await expect(page.locator("main section")).toHaveCount(1);
    await expect(page.locator(`#${kind}`)).toBeVisible();
  };
  await expect(page.locator("main section")).toHaveCount(1);
  await expect(page.locator("#gallery")).toBeVisible();
  // Layout changes retain each photo's confirmation and button inside its bordered card.
  for (const size of [
    { width: 375, height: 812, columns: 1 },
    { width: 844, height: 390, columns: 1 },
    { width: 1080, height: 1920, columns: 3 },
    { width: 1920, height: 1080, columns: 5 },
    { width: 3840, height: 2160, columns: 5 },
  ]) {
    await page.setViewportSize(size);
    const grid = page.locator(".gallery-grid");
    await expect.poll(() => grid.evaluate(element => getComputedStyle(element).gridTemplateColumns.split(" ").length)).toBe(size.columns);
    for (const card of await grid.locator("li").all()) {
      await expect(card).toHaveCSS("border-top-width", "1px");
      await expect(card.locator("img")).toBeVisible();
      await expect(card.getByRole("button", { name: "Remove", exact: true })).toBeVisible();
    }
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= document.documentElement.clientWidth)).toBe(true);
    if (size.columns > 1 && size.width < 3840) {
      await page.screenshot({ path: testInfo.outputPath(`admin-gallery-${size.columns}-columns.png`), fullPage: true });
    }
  }
  await page.setViewportSize({ width: 375, height: 812 });
  const portfolio = (await page.getByRole("link", { name: "Back to the portfolio", exact: true }).boundingBox())!;
  const identity = (await page.locator(".admin-identity").boundingBox())!;
  const logout = (await page.getByRole("button", { name: "Sign out", exact: true }).boundingBox())!;
  const editors = (await navigation.boundingBox())!;
  expect(portfolio.y + portfolio.height).toBeLessThanOrEqual(identity.y);
  expect(identity.y + identity.height).toBeLessThanOrEqual(logout.y);
  expect(logout.y + logout.height).toBeLessThanOrEqual(editors.y);
  await page.screenshot({ path: testInfo.outputPath("admin-gallery-mobile.png"), fullPage: true });
  await select("shows");
  await select("videos");
  await page.goBack();
  await expect(page.locator("#shows")).toBeVisible();
  await page.goForward();
  await expect(page.locator("#videos")).toBeVisible();
  const publishedVideo = testInfo.project.name === "chromium" ? "RBwe-c9T5sY" : "73KJzjvW_1Q";
  const publishedPhoto = `published_${testInfo.project.name}.jpg`;
  // These records were imported before the browser signed in, rather than added by its forms.
  for (const [kind, label] of [["shows", `Published ${testInfo.project.name}`], ["videos", publishedVideo], ["gallery", publishedPhoto]]) {
    await select(kind);
    const form = page.locator(`#${kind} ${kind === "shows" ? "tbody tr" : "li"}`).filter({ hasText: label }).getByRole("form");
    await expect(form).toBeVisible();
    await form.getByRole("checkbox", { name: "Confirm permanent removal" }).check();
    await form.getByRole("button", { name: "Remove", exact: true }).click();
    await expect(page).toHaveURL(new RegExp(`section=${kind}$`));
    await expect(form).toHaveCount(0);
  }
  expect((await request.get(`/img/about_pics/${publishedPhoto}`)).status()).toBe(404);
  await select("shows");
  const showName = `Browser ${testInfo.project.name} <script>literal</script>`;
  const show = page.getByRole("form", { name: "Shows", exact: true });
  await show.getByLabel("Date (required)").fill("2028-02-29");
  await show.getByLabel("Event name (optional)").fill(showName);
  const submitting = page.waitForRequest(request => request.method() === "POST" && request.url().endsWith("/admin/content"));
  await show.getByRole("button", { name: "Add show" }).click();
  expect((await submitting).headers()["origin"]).toBe(new URL(baseURL!).origin);
  await expect(page.locator("#shows")).toContainText(showName);
  expect(await (await request.get("/api/get_shows")).text()).toContain(showName);
  const table = page.getByRole("table");
  await expect(table).toHaveCount(1);
  const dates = await table.locator("tbody time").evaluateAll(nodes => nodes.map(node => node.getAttribute("datetime")!));
  expect(dates).toEqual([...dates].sort().reverse());
  await expect(table.locator("tbody tr").filter({ hasText: showName }).getByRole("button", { name: "Remove", exact: true })).toBeVisible();
  await expect(page.getByRole("region", { name: /Published shows/ })).toHaveAttribute("tabindex", "0");
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= document.documentElement.clientWidth)).toBe(true);
  await page.screenshot({ path: testInfo.outputPath("admin-shows-table.png"), fullPage: true });
  await page.setViewportSize({ width: 1920, height: 1080 });
  await page.screenshot({ path: testInfo.outputPath("admin-shows-desktop.png"), fullPage: true });
  await page.setViewportSize({ width: 375, height: 812 });

  await select("videos");
  const videoId = testInfo.project.name === "chromium" ? "3uh_Jc1prRQ" : "dQw4w9WgXcQ";
  await page.getByLabel("YouTube URL (required)").fill(`https://youtu.be/${videoId}`);
  await page.getByRole("button", { name: "Add video" }).click();
  await expect(page.locator("#videos")).toContainText(videoId);
  expect(await (await request.get("/api/get_media_videos")).text()).toContain(`/embed/${videoId}`);

  await select("gallery");
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

  await page.screenshot({ path: testInfo.outputPath("admin-published-gallery.png"), fullPage: true });
  for (const [kind, label] of [["shows", showName], ["videos", videoId], ["gallery", normalizedName]]) {
    await select(kind);
    const form = page.locator(`#${kind} ${kind === "shows" ? "tbody tr" : "li"}`).filter({ hasText: label }).getByRole("form");
    const confirmation = form.getByRole("checkbox", { name: "Confirm permanent removal" });
    await expect(confirmation).not.toBeChecked();
    await expect(confirmation).toHaveAttribute("required", "");
    await confirmation.focus();
    await page.keyboard.press("Space");
    await form.getByRole("button", { name: "Remove", exact: true }).click();
    await expect(form).toHaveCount(0);
  }
  await page.screenshot({ path: testInfo.outputPath("admin-section.png"), fullPage: true });
  expect((await request.get(imageUrl)).status()).toBe(404);
  expect(await (await request.get("/api/get_shows")).text()).not.toContain(showName);
  expect(await (await request.get("/api/get_media_videos")).text()).not.toContain(videoId);
  await page.getByRole("button", { name: "Sign out", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Administration sign-in", exact: true })).toBeVisible();
});
