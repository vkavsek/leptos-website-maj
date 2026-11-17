import { test, expect } from "@playwright/test";

const original = "/img/bg/bg_smallest.webp";
const smaller = "/img/bg/background_1716.webp";
const desktop = { width: 1920, height: 1080 };
const mobile = { width: 390, height: 844 };
const fast = { effectiveType: "4g", saveData: false };
const scenarios = [
  { name: "fast desktop", viewport: desktop, connection: fast, expected: original },
  { name: "slow 4K desktop", viewport: { width: 3840, height: 2160 }, connection: { ...fast, effectiveType: "3g" }, expected: original },
  { name: "data-saving QHD desktop", viewport: { width: 2560, height: 1440 }, connection: { ...fast, saveData: true }, expected: original },
  { name: "scaled 4K desktop", viewport: desktop, deviceScaleFactor: 2, connection: { ...fast, effectiveType: "3g" }, expected: original },
  { name: "175 percent scaled 4K desktop", viewport: { width: 2194, height: 1234 }, deviceScaleFactor: 1.75, connection: { ...fast, saveData: true }, expected: original },
  ...["slow-2g", "2g", "3g"].map(effectiveType => ({
    name: `${effectiveType} desktop`, viewport: desktop,
    connection: { effectiveType, saveData: false }, expected: smaller,
  })),
  { name: "data-saving desktop", viewport: desktop, connection: { ...fast, saveData: true }, expected: smaller },
  { name: "desktop without connection API", viewport: desktop, connection: undefined, expected: original },
  { name: "mobile without connection API", viewport: mobile, connection: undefined, expected: smaller },
  { name: "fast landscape phone", viewport: { width: 844, height: 390 }, connection: fast, hasTouch: true, expected: smaller },
  { name: "desktop without JavaScript", viewport: desktop, connection: undefined, javaScriptEnabled: false, expected: original },
  { name: "mobile without JavaScript", viewport: mobile, connection: undefined, javaScriptEnabled: false, expected: smaller },
];

for (const scenario of scenarios) {
  test.describe(scenario.name, () => {
    test.use({
      viewport: scenario.viewport,
      deviceScaleFactor: "deviceScaleFactor" in scenario ? scenario.deviceScaleFactor : 1,
      hasTouch: "hasTouch" in scenario ? scenario.hasTouch : false,
      javaScriptEnabled: "javaScriptEnabled" in scenario ? scenario.javaScriptEnabled : true,
    });

    test("downloads only the selected background", async ({ page }) => {
      // Control the capability rather than depending on the test host's connection.
      await page.addInitScript(connection => {
        Object.defineProperty(navigator, "connection", { configurable: true, value: connection });
      }, scenario.connection);
      const requests: string[] = [];
      page.on("request", request => {
        const path = new URL(request.url()).pathname;
        if (path === original || path === smaller) requests.push(path);
      });
      const response = page.waitForResponse(response => new URL(response.url()).pathname === scenario.expected);
      await page.goto("/");
      expect((await response).ok()).toBe(true);
      await expect(page.locator(".bg-wrapper")).toHaveCSS("background-image", `url("${new URL(scenario.expected, page.url()).href}")`);
      expect(requests).toEqual([scenario.expected]);
    });
  });
}
