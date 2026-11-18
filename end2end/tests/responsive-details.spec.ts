import { test, expect, type Page } from "@playwright/test";

test("portrait play button follows player height on phones and large screens", async ({ page }) => {
  for (const size of [
    { width: 390, height: 700, font: 16 },
    { width: 1080, height: 1920, font: 16 },
    { width: 2160, height: 3840, font: 16 },
    { width: 1080, height: 1920, font: 24 },
  ]) {
    await page.setViewportSize(size);
    await ready(page, "/");
    await page.addStyleTag({ content: `html { font-size: ${size.font}px; }` });
    await page.mouse.move(0, 0);
    const player = page.locator(".audio-player");
    await player.evaluate(element => Promise.all(element.getAnimations({ subtree: true }).map(animation => animation.finished)));
    const panel = (await player.boundingBox())!;
    const button = page.getByRole("button", { name: "Play", exact: true });
    const bounds = (await button.boundingBox())!;
    expect(bounds.height / panel.height).toBeCloseTo(0.8, 2);
    expect(bounds.width).toBeCloseTo(bounds.height, 0);
    await button.hover();
    await button.evaluate(element => Promise.all(element.getAnimations().map(animation => animation.finished)));
    const hovered = (await button.boundingBox())!;
    expect(hovered.y).toBeGreaterThanOrEqual(panel.y);
    expect(hovered.y + hovered.height).toBeLessThanOrEqual(panel.y + panel.height);
    await page.mouse.move(0, 0);
    expect(await page.locator(".time").evaluate(element => parseFloat(getComputedStyle(element).marginBottom))).toBeGreaterThan(0);
  }
});

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
      expect(nav.y - player.y - player.height).toBeCloseTo(32, 0);
      expect(viewport.height - nav.y - nav.height).toBeGreaterThanOrEqual(31);
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

test("landscape navigation does not overflow while icons spin", async ({ page }) => {
  await page.setViewportSize({ width: 1920, height: 1080 });
  await ready(page, "/");
  const nav = page.locator(".nav-links");
  const player = (await page.locator(".audio-player").boundingBox())!;
  expect(player.y).toBeCloseTo(1080 * 0.04 - 2, 0);
  const bounds = (await nav.boundingBox())!;
  expect(bounds.y - player.y - player.height).toBeCloseTo(32, 0);
  expect(1080 - bounds.y - bounds.height).toBeGreaterThanOrEqual(53);
  const sampleOverflow = () => nav.evaluate(async element => {
    let overflow = false;
    const start = performance.now();
    while (performance.now() - start < 250) {
      overflow ||= element.scrollHeight > element.clientHeight || element.scrollWidth > element.clientWidth;
      await new Promise(requestAnimationFrame);
    }
    return overflow;
  });
  for (const link of await nav.locator("a").all()) {
    await link.hover();
    expect(await sampleOverflow()).toBe(false);
    await page.mouse.move(960, 540);
    expect(await sampleOverflow()).toBe(false);
  }
  await page.setViewportSize({ width: 320, height: 180 });
  for (const link of await nav.locator("a").all()) {
    await link.focus();
    await expect(link).toBeInViewport({ ratio: 0.99 });
  }
});

test("short album grids center vertically and longer grids remain reachable", async ({ page }) => {
  for (const viewport of [{ width: 3840, height: 2160 }, { width: 1920, height: 1080 }]) {
    await page.setViewportSize(viewport);
    await ready(page, "/discography");
    await page.locator(".discography-link img").evaluateAll(images => Promise.all(
      images.map(image => (image as HTMLImageElement).decode())
    ));
    const area = (await page.locator("#disco-wrap").boundingBox())!;
    const albums = (await page.locator("#album-wrap").boundingBox())!;
    expect(albums.y + albums.height / 2).toBeCloseTo(area.y + area.height / 2, 0);
    await page.locator("#album-wrap").evaluate(element => {
      const album = element.firstElementChild!;
      for (let index = 0; index < 8; index++) element.append(album.cloneNode(true));
    });
    const links = page.locator(".discography-link");
    await expect(links).toHaveCount(10);
    for (const link of [links.first(), links.last()]) {
      await link.scrollIntoViewIfNeeded();
      await expect(link).toBeInViewport({ ratio: 0.99 });
    }
  }
});

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
    // Match the other caption check's tolerance for subpixel intersection rounding at zoom.
    await expect(link).toBeInViewport({ ratio: 0.99 });
  }
});

for (const viewport of [
  { width: 1280, height: 720 }, { width: 640, height: 360 },
  { width: 320, height: 180 }, { width: 390, height: 700 },
]) {
  for (const fontSize of ["16px", "32px"]) {
    test(`show rows remain readable at ${viewport.width}×${viewport.height} with ${fontSize} text`, async ({ page }) => {
      await page.setViewportSize(viewport);
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
    });
  }
}


test("home contact icons fit the viewport and remain separated", async ({ page }) => {
  for (const viewport of [{ width: 1280, height: 720 }, { width: 390, height: 700 }, { width: 320, height: 180 }]) {
    await page.setViewportSize(viewport);
    await ready(page, "/");
    const icons = page.locator(".all-contacts .contacts-img");
    await expect(icons).toHaveCount(5);
    for (const icon of await icons.all()) {
      await icon.scrollIntoViewIfNeeded();
      const box = (await icon.boundingBox())!;
      expect(box.width).toBeGreaterThanOrEqual(viewport.width > viewport.height ? 15.5 : 19.5);
      expect(box.x).toBeGreaterThanOrEqual(0);
      expect(box.x + box.width).toBeLessThanOrEqual(viewport.width);
      await expect(icon).toBeInViewport();
    }
    const boxes = await icons.evaluateAll(elements => elements.map(element => {
      const rect = element.getBoundingClientRect();
      return { center: rect.x + rect.width / 2 };
    }));
    for (let index = 1; index < boxes.length; index++) {
      expect(boxes[index].center - boxes[index - 1].center).toBeGreaterThanOrEqual(24);
    }
  }
});

test("landscape navigation keeps its top gap and reserves bottom space on desktop and 4K", async ({ page }, testInfo) => {
  for (const viewport of [
    { width: 1280, height: 720, bottom: 32 },
    { width: 1920, height: 1080, bottom: 64 },
    { width: 3840, height: 2160, bottom: 180 },
  ]) {
    await page.setViewportSize(viewport);
    await ready(page, "/");
    const nav = page.locator(".nav-links");
    const bounds = (await nav.boundingBox())!;
    const player = (await page.locator(".audio-player").boundingBox())!;
    const last = (await nav.locator("a").last().boundingBox())!;
    expect(bounds.y - player.y - player.height).toBeCloseTo(32, 0);
    expect(viewport.height - bounds.y - bounds.height).toBeGreaterThanOrEqual(viewport.bottom);
    expect(viewport.height - last.y - last.height).toBeGreaterThanOrEqual(viewport.bottom);
    expect(await nav.evaluate(element => element.scrollHeight <= element.clientHeight)).toBe(true);
    await page.screenshot({ path: testInfo.outputPath(`navigation-${viewport.width}.png`) });
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
