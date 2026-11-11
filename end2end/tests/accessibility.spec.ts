import { test, expect, type Locator, type Page } from "@playwright/test";

async function ready(page: Page, path = "/") {
  await page.goto(path);
  await expect.poll(() => page.locator("audio").evaluate(audio => (audio as HTMLAudioElement).volume)).toBe(0.5);
}

async function focusStaysInside(page: Page, dialog: Locator, steps: number) {
  for (const key of ["Tab", "Shift+Tab"]) {
    for (let step = 0; step < steps; step++) {
      await page.keyboard.press(key);
      // Native dialogs may let Tab visit browser chrome, but never the inert page.
      await expect.poll(() => dialog.evaluate(element =>
        document.activeElement === document.body || element.contains(document.activeElement)
      )).toBe(true);
      await expect.poll(() => dialog.evaluate(element => element.matches(":modal"))).toBe(true);
    }
  }
  await dialog.locator("button").first().focus();
  await page.locator(".nav-links a").first().evaluate(element => (element as HTMLElement).focus());
  await expect.poll(() => dialog.evaluate(element => element.contains(document.activeElement))).toBe(true);
}

test("audio sliders work with arrows, Home and End", async ({ page }) => {
  await ready(page);
  const play = page.getByRole("button", { name: "Play", exact: true });
  await play.focus();
  const position = page.getByRole("slider", { name: "Playback position" });
  await expect(position).toBeEnabled();
  await play.press("Tab");
  await expect(position).toBeFocused();
  await position.press("Home");
  await position.press("ArrowRight");
  await expect.poll(() => page.locator("audio").evaluate(audio => (audio as HTMLAudioElement).currentTime)).toBeCloseTo(1, 0);
  await position.press("End");
  await expect.poll(() => page.locator("audio").evaluate(audio => {
    const media = audio as HTMLAudioElement;
    return Math.abs(media.currentTime - Math.floor(media.duration));
  })).toBeLessThan(1);

  await position.press("Tab");
  await expect(page.getByRole("button", { name: "Mute", exact: true })).toBeFocused();
  await page.keyboard.press("Tab");
  const volume = page.getByRole("slider", { name: "Volume", exact: true });
  await expect(volume).toBeFocused();
  await volume.press("Home");
  await expect.poll(() => page.locator("audio").evaluate(audio => (audio as HTMLAudioElement).volume)).toBe(0);
  await volume.press("ArrowUp");
  await expect.poll(() => page.locator("audio").evaluate(audio => (audio as HTMLAudioElement).volume)).toBeCloseTo(0.01, 2);
  await volume.press("End");
  await expect.poll(() => page.locator("audio").evaluate(audio => (audio as HTMLAudioElement).volume)).toBe(1);
  await volume.press("Shift+Tab");
  await page.keyboard.press("Space");
  await expect(volume).toHaveValue("0");
  await page.keyboard.press("Space");
  await expect(volume).toHaveValue("1");
});

test("contact dialog contains focus and restores it after Escape or closing", async ({ page }) => {
  await ready(page);
  const trigger = page.getByRole("button", { name: "Send me an email" });
  const dialog = page.getByRole("dialog", { name: "Contact Maj Kavšek" });
  await trigger.focus();
  await trigger.press("Enter");
  await expect(dialog).toBeVisible();
  await expect(page.getByLabel("Enter your name:")).toBeFocused();
  await expect(dialog).toHaveJSProperty("open", true);
  await focusStaysInside(page, dialog, 8);
  await page.keyboard.press("Escape");
  await expect(dialog).toBeHidden();
  await expect(trigger).toBeFocused();
  await trigger.press("Space");
  await dialog.getByRole("button", { name: "Close contact form" }).press("Enter");
  await expect(dialog).toBeHidden();
  await expect(trigger).toBeFocused();
});

test("external-link dialog supports keyboard cancellation and reopening", async ({ page }) => {
  await ready(page);
  const trigger = page.getByRole("link", { name: "A link to Facebook" });
  const dialog = page.getByRole("dialog", { name: "Open external website" });
  await trigger.focus();
  await trigger.press("Enter");
  await expect(dialog.getByRole("button", { name: "Back", exact: true })).toBeFocused();
  await focusStaysInside(page, dialog, 4);
  await page.keyboard.press("Escape");
  await expect(dialog).toBeHidden();
  await expect(trigger).toBeFocused();
  await trigger.press("Enter");
  await dialog.getByRole("button", { name: "Back", exact: true }).press("Enter");
  await expect(dialog).toBeHidden();
  await expect(trigger).toBeFocused();
});

test("external tabs cannot access the original tab", async ({ page, context }) => {
  await context.route("https://www.facebook.com/**", route => route.fulfill({
    contentType: "text/html", body: "<title>External fixture</title>",
  }));
  await ready(page);
  await page.getByRole("link", { name: "A link to Facebook" }).click();
  const opened = context.waitForEvent("page");
  await page.getByRole("button", { name: "Open in new tab", exact: true }).click();
  const external = await opened;
  try {
    await external.waitForLoadState();
    await expect(external).toHaveURL("https://www.facebook.com/majkavsek");
    expect(await external.evaluate(() => window.opener)).toBeNull();
    await expect(page).toHaveURL(/\/$/);
    await expect(page.getByRole("dialog", { name: "Open external website" })).toBeHidden();
  } finally {
    await external.close();
  }
});

test("Media displays video loading errors after navigation", async ({ page }) => {
  await ready(page);
  await page.route("**/api/get_media_videos*", route => route.fulfill({
    status: 500, contentType: "application/json",
    headers: { serverfnerror: "/api/get_media_videos" },
    body: JSON.stringify({ SerdeJson: "Invalid video content fixture" }),
  }));
  await page.locator('.nav-links a[href="/media"]').click();
  await expect(page.locator("#media-contents .error-wrap")).toContainText("Invalid video content fixture");
  await expect(page.locator("#media-contents .yt-video")).toHaveCount(0);
});

test("gallery buttons open a modal, navigate photos and restore focus", async ({ page }) => {
  await ready(page, "/about_me");
  const trigger = page.getByRole("button", { name: "Open gallery photo 1", exact: true });
  const dialog = page.getByRole("dialog", { name: "Photo viewer" });
  await trigger.focus();
  await trigger.press("Space");
  await expect(dialog).toBeVisible();
  await expect(dialog.getByRole("button", { name: "Close photo" })).toBeFocused();
  const image = dialog.getByRole("img", { name: "Enlarged gallery photo" });
  const first = await image.getAttribute("src");
  await dialog.getByRole("button", { name: "Next photo" }).press("Enter");
  await expect(image).not.toHaveAttribute("src", first!);
  await dialog.getByRole("button", { name: "Previous photo" }).press("Enter");
  await expect(image).toHaveAttribute("src", first!);
  await dialog.getByRole("button", { name: "Next photo" }).click();
  await expect(dialog).toBeVisible();
  await expect(image).not.toHaveAttribute("src", first!);
  await dialog.getByRole("button", { name: "Previous photo" }).click();
  await expect(image).toHaveAttribute("src", first!);
  await focusStaysInside(page, dialog, 4);
  await page.keyboard.press("Escape");
  await expect(dialog).toBeHidden();
  await expect(trigger).toBeFocused();
  await trigger.press("Enter");
  await dialog.getByRole("button", { name: "Close photo" }).press("Enter");
  await expect(dialog).toBeHidden();
  await expect(trigger).toBeFocused();
  await trigger.press("Enter");
  await page.mouse.click(1, 1);
  await expect(dialog).toBeHidden();
  await expect(trigger).toBeFocused();
});

test("backdrop fallback ignores padding and inside-to-outside drags", async ({ page }) => {
  await page.addInitScript(() => { delete (HTMLDialogElement.prototype as any).closedBy; });
  await ready(page);
  const trigger = page.getByRole("button", { name: "Send me an email" });
  const dialog = page.getByRole("dialog", { name: "Contact Maj Kavšek" });
  await trigger.click();
  // Disable native light dismiss as well as its reflected property, exercising the fallback.
  await dialog.evaluate(element => element.setAttribute("closedby", "closerequest"));
  const box = (await dialog.boundingBox())!;
  await page.mouse.click(box.x + 4, box.y + 4);
  await expect(dialog).toBeVisible();
  await page.mouse.move(box.x + 4, box.y + 4);
  await page.mouse.down();
  await page.mouse.move(1, 1);
  await page.mouse.up();
  await expect(dialog).toBeVisible();
  await page.mouse.click(1, 1);
  await expect(dialog).toBeHidden();
  await expect(trigger).toBeFocused();
});

test("mail failure keeps entered text; success closes and resets the form", async ({ page }) => {
  await ready(page);
  const action = await page.locator(".email-form").getAttribute("action");
  let succeed = false;
  await page.route(`**${action}`, route => route.fulfill(succeed ? {
    status: 200, contentType: "application/json", body: "null",
  } : {
    status: 400, contentType: "text/plain", headers: { serverfnerror: action! },
    body: "Args|Check this test message.",
  }));
  const trigger = page.getByRole("button", { name: "Send me an email" });
  const dialog = page.getByRole("dialog", { name: "Contact Maj Kavšek" });
  await trigger.click();
  await page.getByLabel("Enter your name:").fill("Test sender");
  await page.getByLabel("Enter your email address:").fill("test@example.com");
  await page.getByLabel("Subject:", { exact: true }).fill("Keyboard test");
  await page.getByLabel("Your message", { exact: true }).fill("Test message, never sent.");
  await page.getByRole("button", { name: "Send", exact: true }).click();
  await expect(dialog.getByRole("alert")).toContainText("Check this test message.");
  await expect(page.getByLabel("Your message", { exact: true })).toHaveValue("Test message, never sent.");
  await expect(dialog).toBeVisible();
  succeed = true;
  await page.getByRole("button", { name: "Send", exact: true }).click();
  await expect(dialog).toBeHidden();
  await expect(trigger).toBeFocused();
  await trigger.click();
  await expect(page.getByLabel("Your message", { exact: true })).toHaveValue("");
});

test("portrait dialogs fit the viewport and close with Escape", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 700 });
  await ready(page);
  const trigger = page.getByRole("button", { name: "Send me an email" });
  await trigger.press("Enter");
  const dialog = page.getByRole("dialog", { name: "Contact Maj Kavšek" });
  await expect(dialog).toBeVisible();
  const box = (await dialog.boundingBox())!;
  expect(box.x).toBeGreaterThanOrEqual(0);
  expect(box.y).toBeGreaterThanOrEqual(0);
  expect(box.x + box.width).toBeLessThanOrEqual(390);
  expect(box.y + box.height).toBeLessThanOrEqual(700);
  await page.keyboard.press("Escape");
  await expect(trigger).toBeFocused();
});
