import { test, expect } from "@playwright/test";

test("gallery loads small thumbnails and requests originals only on opening", async ({ page }, testInfo) => {
  const originalRequests: string[] = [];
  page.on("request", request => {
    if (new URL(request.url()).pathname.startsWith("/img/about_pics/")) originalRequests.push(request.url());
  });
  await page.goto("/about_me");
  await expect.poll(() => page.locator("audio").evaluate(audio => (audio as HTMLAudioElement).volume)).toBe(0.5);
  const thumbnails = page.locator(".gallery-thumbnail img");
  await expect(thumbnails).toHaveCount(12);
  for (let index = 0; index < 12; index++) {
    const image = thumbnails.nth(index);
    await expect(image).toHaveAttribute("src", /^\/img\/about_thumbnails\/.+\.webp$/);
    await expect(image).toHaveAttribute("width", "480");
    await expect(image).toHaveAttribute("height", "270");
    await expect(image).toHaveAttribute("loading", index < 4 ? "eager" : "lazy");
  }
  await expect.poll(() => thumbnails.first().evaluate(image => (image as HTMLImageElement).naturalWidth)).toBe(480);
  const size = (await thumbnails.first().boundingBox())!;
  expect(size.width / size.height).toBeCloseTo(16 / 9, 1);
  expect(originalRequests).toHaveLength(0);
  await page.screenshot({ path: testInfo.outputPath("gallery-thumbnails.png") });
  await page.getByRole("button", { name: "Open gallery photo 1", exact: true }).click();
  const fullImage = page.getByRole("dialog", { name: "Photo viewer" }).getByRole("img");
  await expect(fullImage).toHaveAttribute("src", /^\/img\/about_pics\//);
  await expect.poll(() => fullImage.evaluate(image => (image as HTMLImageElement).naturalWidth)).toBeGreaterThan(480);
  expect(originalRequests).toHaveLength(1);
});

test("media events synchronize playback without a polling timer", async ({ page }) => {
  await page.addInitScript(() => {
    (window as any).testIntervalDelays = [];
    const original = window.setInterval.bind(window);
    window.setInterval = ((handler: TimerHandler, timeout?: number, ...args: any[]) => {
      (window as any).testIntervalDelays.push(timeout);
      return original(handler, timeout, ...args);
    }) as typeof window.setInterval;
  });
  await page.goto("/");
  const audio = page.locator("audio");
  await expect.poll(() => audio.evaluate(element => (element as HTMLAudioElement).volume)).toBe(0.5);
  await page.getByRole("button", { name: "Play", exact: true }).focus();
  const position = page.getByRole("slider", { name: "Playback position" });
  await expect(position).toBeEnabled();
  await page.getByRole("button", { name: "Play", exact: true }).click();
  await expect(page.getByRole("button", { name: "Pause", exact: true })).toBeVisible();
  await audio.evaluate(element => (element as HTMLAudioElement).pause());
  await expect(page.getByRole("button", { name: "Play", exact: true })).toBeVisible();
  await audio.evaluate(element => { (element as HTMLAudioElement).currentTime = 12.25; });
  await expect(position).toHaveValue("12");
  await expect(page.locator(".current-time")).toHaveText("00:12");
  await audio.evaluate(element => {
    Object.defineProperty(element, "duration", { configurable: true, value: Infinity });
    element.dispatchEvent(new Event("durationchange"));
  });
  await expect(position).toBeDisabled();
  await audio.evaluate(element => {
    delete (element as any).duration;
    element.dispatchEvent(new Event("durationchange"));
  });
  await expect(position).toBeEnabled();
  await page.getByRole("button", { name: "Play", exact: true }).click();
  await expect(page.getByRole("button", { name: "Pause", exact: true })).toBeVisible();
  await audio.evaluate(element => {
    const media = element as HTMLAudioElement;
    media.currentTime = media.duration - 0.05;
  });
  await expect.poll(() => audio.evaluate(element => (element as HTMLAudioElement).ended)).toBe(true);
  await expect(page.getByRole("button", { name: "Play", exact: true })).toBeVisible();
  expect(await page.evaluate(() => (window as any).testIntervalDelays)).not.toContain(10);
});

test("a rejected play request leaves the control paused without an unhandled rejection", async ({ page }) => {
  const errors: string[] = [];
  page.on("pageerror", error => errors.push(error.message));
  await page.goto("/");
  const audio = page.locator("audio");
  await expect.poll(() => audio.evaluate(element => (element as HTMLAudioElement).volume)).toBe(0.5);
  await audio.evaluate(element => {
    (element as HTMLAudioElement).play = () => Promise.reject(new DOMException("Test rejection", "NotAllowedError"));
  });
  await page.getByRole("button", { name: "Play", exact: true }).click();
  await page.evaluate(() => new Promise<void>(resolve => requestAnimationFrame(() => resolve())));
  await expect(page.getByRole("button", { name: "Play", exact: true })).toBeVisible();
  expect(await audio.evaluate(element => (element as HTMLAudioElement).paused)).toBe(true);
  expect(errors).toEqual([]);
});
