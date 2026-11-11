import { test, expect, type Page } from "@playwright/test";

async function ready(page: Page, path: string) {
  await page.goto(path);
  await expect.poll(() => page.locator("audio").evaluate(audio => (audio as HTMLAudioElement).volume)).toBe(0.5);
  await page.evaluate(() => document.fonts.ready);
}

for (const viewport of [
  { width: 1280, height: 720 },
  { width: 1920, height: 1080 },
  { width: 1024, height: 500 },
  { width: 640, height: 360 },
  { width: 390, height: 700 },
  { width: 320, height: 180 },
]) {
  test(`responsive details retain the layout at ${viewport.width}×${viewport.height}`, async ({ page }) => {
    await page.setViewportSize(viewport);
    await page.route(/youtube(?:-nocookie)?\.com/, route => route.abort());
    await ready(page, "/media");
    const video = page.locator(".yt-video").first();
    const frame = (await video.boundingBox())!;
    expect(frame.width / frame.height).toBeCloseTo(4 / 3, 2);
    expect(frame.width).toBeGreaterThan(Math.min(240, viewport.width / 2, viewport.height * 0.7));
    expect(frame.height).toBeLessThanOrEqual(viewport.height * 0.6 + 1);
    await expect(video).toHaveAttribute("allowfullscreen", "");
    const player = (await page.locator(".audio-player").boundingBox())!;
    const nav = (await page.locator(".nav-links").boundingBox())!;
    if (viewport.width > viewport.height) {
      expect(nav.y - player.y - player.height).toBeCloseTo(16, 0);
      expect(nav.x + nav.width).toBeLessThan(viewport.width / 5);
    } else {
      // Preserve the original portrait navigation header and player footer.
      expect(nav.y).toBe(0);
      expect(player.y + player.height).toBeCloseTo(viewport.height, 0);
    }
    await ready(page, "/about_me");
    const images = page.locator(".gallery-thumbnail img");
    await expect(images).toHaveCount(12);
    const first = (await images.nth(0).boundingBox())!;
    const second = (await images.nth(1).boundingBox())!;
    expect(first.width / first.height).toBeCloseTo(16 / 9, 2);
    const gap = viewport.width > viewport.height ? second.y - first.y - first.height : second.x - first.x - first.width;
    expect(gap).toBeGreaterThanOrEqual(15);
    // The gallery stays on the right in landscape, above the text in portrait.
    const text = (await page.locator(".about-text").boundingBox())!;
    if (viewport.width > viewport.height) expect(first.x).toBeGreaterThan(text.x);
    else expect(first.y).toBeLessThan(text.y);
    await ready(page, "/discography");
    for (const link of await page.locator(".discography-link").all()) {
      await link.scrollIntoViewIfNeeded();
      await expect(link).toBeInViewport();
      const box = (await link.boundingBox())!;
      expect(box.x).toBeGreaterThanOrEqual(0);
      expect(box.x + box.width).toBeLessThanOrEqual(viewport.width);
    }
  });
}

test("larger text does not shrink gallery photos or hide album captions", async ({ page }) => {
  await ready(page, "/about_me");
  const images = page.locator(".gallery-thumbnail img");
  const before = (await images.first().boundingBox())!;
  await page.evaluate(() => { document.documentElement.style.fontSize = "32px"; });
  const first = (await images.nth(0).boundingBox())!;
  const second = (await images.nth(1).boundingBox())!;
  expect(first.width).toBeCloseTo(before.width, 0);
  expect(second.y - first.y - first.height).toBeGreaterThanOrEqual(31);
  await ready(page, "/discography");
  await page.evaluate(() => { document.documentElement.style.fontSize = "32px"; });
  for (const link of await page.locator(".discography-link").all()) {
    await link.scrollIntoViewIfNeeded();
    await expect(link).toBeInViewport({ ratio: 1 });
  }
});

test("show rows remain readable at high zoom and with larger text", async ({ page }) => {
  for (const viewport of [
    { width: 1280, height: 720 }, { width: 640, height: 360 },
    { width: 320, height: 180 }, { width: 390, height: 700 },
  ]) {
    await page.setViewportSize(viewport);
    for (const fontSize of ["16px", "32px"]) {
      await ready(page, "/shows/past");
      await page.evaluate(size => { document.documentElement.style.fontSize = size; }, fontSize);
      const rows = page.locator(".show-container:not(#show-container-id)");
      expect(await rows.count()).toBeGreaterThan(0);
      for (const row of [rows.first(), rows.last()]) {
        for (const cell of await row.locator("p").all()) {
          await cell.scrollIntoViewIfNeeded();
          // Allow subpixel rounding at the end of a scroll region.
          await expect(cell).toBeInViewport({ ratio: 0.99 });
        }
      }
      for (const link of await page.locator(".shows-nav a").all()) {
        await link.scrollIntoViewIfNeeded();
        await expect(link).toBeInViewport({ ratio: 0.99 });
      }
    }
  }
});

test("vertical volume slider stays beside mute and inside the hovered player", async ({ page }) => {
  for (const viewport of [
    { width: 1280, height: 720 }, { width: 1920, height: 1080 },
    { width: 640, height: 360 }, { width: 390, height: 700 }, { width: 320, height: 180 },
  ]) {
    await page.setViewportSize(viewport);
    await ready(page, "/");
    const player = page.locator(".audio-player");
    await page.getByRole("button", { name: "Play", exact: true }).focus();
    await player.evaluate(element => Promise.all(element.getAnimations({ subtree: true }).map(animation => animation.finished)));
    await player.hover();
    const volume = page.getByRole("slider", { name: "Volume", exact: true });
    await expect(volume).toBeVisible();
    const before = (await volume.boundingBox())!;
    expect(before.height).toBeGreaterThan(before.width);
    expect(before.y).toBeGreaterThanOrEqual(0);
    expect(before.y + before.height).toBeLessThanOrEqual(viewport.height);
    const mute = (await page.getByRole("button", { name: "Mute", exact: true }).boundingBox())!;
    const panel = (await player.boundingBox())!;
    expect(before.x).toBeGreaterThanOrEqual(mute.x + mute.width);
    expect(before.x).toBeGreaterThanOrEqual(panel.x);
    expect(before.y).toBeGreaterThanOrEqual(panel.y);
    expect(before.x + before.width).toBeLessThanOrEqual(panel.x + panel.width);
    expect(before.y + before.height).toBeLessThanOrEqual(panel.y + panel.height);
    await volume.hover();
    await player.evaluate(element => Promise.all(element.getAnimations({ subtree: true }).map(animation => animation.finished)));
    const after = (await volume.boundingBox())!;
    expect(after.x).toBeCloseTo(before.x, 0);
    expect(after.y).toBeCloseTo(before.y, 0);
    await volume.press("Home");
    await volume.press("ArrowUp");
    await expect(volume).toHaveValue("0.01");
    await volume.press("End");
    await expect(volume).toHaveValue("1");
  }
});

test("home contact icons are larger and remain within the viewport", async ({ page }) => {
  for (const viewport of [{ width: 1280, height: 720 }, { width: 390, height: 700 }, { width: 320, height: 180 }]) {
    await page.setViewportSize(viewport);
    await ready(page, "/");
    const icons = page.locator(".all-contacts .contacts-img");
    await expect(icons).toHaveCount(5);
    for (const icon of await icons.all()) {
      await icon.scrollIntoViewIfNeeded();
      const box = (await icon.boundingBox())!;
      expect(box.width).toBeGreaterThanOrEqual(23.5);
      expect(box.x).toBeGreaterThanOrEqual(0);
      expect(box.x + box.width).toBeLessThanOrEqual(viewport.width);
      await expect(icon).toBeInViewport();
    }
  }
});

for (const viewport of [
  { width: 1280, height: 720 }, { width: 1280, height: 400 },
  { width: 640, height: 360 }, { width: 320, height: 180 }, { width: 390, height: 700 },
]) {
  test(`album detail contents remain reachable at ${viewport.width}×${viewport.height}`, async ({ page }) => {
    await page.setViewportSize(viewport);
    for (const album of ["minorflaw", "galaterna"]) {
      await ready(page, `/discography/${album}`);
      const content = page.locator("#disco-display-components");
      await expect(content.locator("h1")).toBeInViewport();
      for (const paragraph of await content.locator(".album-desc-wrap p").all()) {
        await paragraph.scrollIntoViewIfNeeded();
        await expect(paragraph).toBeInViewport();
      }
      const back = content.getByRole("link", { name: "Back", exact: true });
      await back.scrollIntoViewIfNeeded();
      await expect(back).toBeInViewport();
      const artwork = content.locator(".album-img-wrap > img");
      await artwork.scrollIntoViewIfNeeded();
      await expect(artwork).toBeInViewport();
      for (const link of await content.locator(".album-logo").all()) {
        await link.scrollIntoViewIfNeeded();
        await expect(link).toBeInViewport();
      }
    }
  });
}
