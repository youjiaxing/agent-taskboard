import assert from "node:assert/strict";
import { createBoardSession } from "./board-harness.mjs";

const session = await createBoardSession();
const appearanceKey = "agent-taskboard-browser-appearance";
const settingsKey = "agent-taskboard-browser-client-settings";
const cases = [
  [{ language: "en" }, "system"],
  ...["system", "light", "dark"].map((appearancePreference) => [{ language: "en", appearancePreference }, appearancePreference]),
  ...["warm", "future-theme", null, 42, true, ["dark"], { dark: null }]
    .map((appearancePreference) => [{ language: "en", appearancePreference }, "light"]),
];
try {
  await session.configurePage();
  // Let first-launch defaults settle before seeding persisted restart cases.
  await session.page.waitForSelector(".chrome");
  for (const viewport of [{ width: 1280, height: 720 }, { width: 390, height: 844 }]) {
    await session.page.setViewportSize(viewport);
    await session.page.emulateMedia({ colorScheme: "dark" });
    for (const [stored, expected] of cases) {
      await session.page.evaluate(({ appearanceKey, settingsKey, stored }) => {
        localStorage.setItem(appearanceKey, JSON.stringify(stored));
        localStorage.setItem(settingsKey, JSON.stringify({ autoFocusNewRun: false }));
      }, { appearanceKey, settingsKey, stored });
      for (let restart = 0; restart < 2; restart++) {
        await session.page.reload({ waitUntil: "domcontentloaded" });
        await session.page.waitForSelector(".chrome");
        assert.equal(await session.page.getAttribute("html", "lang"), "en", JSON.stringify(stored));
        assert.equal(await session.page.getAttribute("html", "data-theme"), expected === "system" ? "dark" : expected, JSON.stringify(stored));
        assert.equal(await session.page.evaluate((key) => JSON.parse(localStorage.getItem(key)).autoFocusNewRun, settingsKey), false);
      }
      if (viewport.width < 640) {
        await session.page.click("button[data-act='mobile-drawer']");
        await session.page.click("[data-act='mobile-appearance-entry']");
        assert.deepEqual(await session.page.$$eval("[data-dialog-id='mobile-drawer'] [data-act='appearance']", (nodes) => nodes.map((node) => node.textContent.trim())), ["Follow system", "Light", "Dark"]);
        await session.page.click("[data-dialog-id='mobile-drawer'] [data-act='appearance'][data-id='dark']");
      } else {
        await session.page.click("button[data-act='appearance-menu']");
        assert.deepEqual(await session.page.$$eval(".appearance-menu [role='menuitemradio']", (nodes) => nodes.map((node) => node.textContent.trim())), ["Follow system", "Light", "Dark"]);
        await session.page.click(".appearance-menu [data-id='dark']");
      }
      await session.page.reload({ waitUntil: "domcontentloaded" });
      await session.page.waitForSelector(".chrome");
      assert.equal(await session.page.getAttribute("html", "data-theme"), "dark");
      assert.deepEqual(await session.page.evaluate((key) => JSON.parse(localStorage.getItem(key)), appearanceKey), { language: "en", appearancePreference: "dark" });
      assert.equal(await session.page.evaluate((key) => JSON.parse(localStorage.getItem(key)).autoFocusNewRun, settingsKey), false);
    }
  }
  const hostAppearance = await session.page.evaluate(async () => (await (await fetch("/rpc", {
    method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify({ op: "snapshot" }),
  })).json()).snapshot.appearance);
  assert.equal(hostAppearance.appearancePreference, "system");
  assert.equal(hostAppearance.language, "zh-CN");
} finally {
  await session.close();
}
console.log("appearance preference storage matrix ok");
