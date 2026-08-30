import { mkdir, readFile, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import pixelmatch from "pixelmatch";
import { PNG } from "pngjs";

const visualBaselineDir = join(dirname(fileURLToPath(import.meta.url)), "baselines");
const visualDiffDir = process.env.VISUAL_DIFF_DIR ?? join("target", "visual-diffs");
const updateVisualBaselines = process.env.UPDATE_VISUAL_BASELINES === "1";
const deterministicNowMs = 1_787_748_507_000;

export async function installDeterministicHostProtocol(page, protocol) {
  await page.addInitScript(({ protocol, nowMs }) => {
    const intervalCallbacks = [];
    Date.now = () => nowMs;
    window.setInterval = (callback, _delay, ...args) => {
      intervalCallbacks.push(() => callback(...args));
      return intervalCallbacks.length;
    };
    window.__RUN_INTERVAL_CALLBACKS__ = () => intervalCallbacks.forEach((callback) => callback());
    window.__HOST_PROTOCOL__ = protocol;
  }, { protocol, nowMs: deterministicNowMs });
}

export function createVisualAssert(page) {
  return async (name) => {
    await page.addStyleTag({
      content: "*, *::before, *::after { animation: none !important; transition: none !important; caret-color: transparent !important; }",
    });
    const actualBuffer = await page.screenshot({ fullPage: false });
    const baselinePath = join(visualBaselineDir, name);
    if (updateVisualBaselines) {
      await mkdir(visualBaselineDir, { recursive: true });
      await writeFile(baselinePath, actualBuffer);
      return;
    }
    let expectedBuffer;
    try {
      expectedBuffer = await readFile(baselinePath);
    } catch {
      throw new Error(`missing visual baseline ${baselinePath}; run with UPDATE_VISUAL_BASELINES=1`);
    }
    const actual = PNG.sync.read(actualBuffer);
    const expected = PNG.sync.read(expectedBuffer);
    if (actual.width !== expected.width || actual.height !== expected.height) {
      throw new Error(`visual baseline dimensions changed for ${name}: ${expected.width}x${expected.height} -> ${actual.width}x${actual.height}`);
    }
    const diff = new PNG({ width: actual.width, height: actual.height });
    const different = pixelmatch(expected.data, actual.data, diff.data, actual.width, actual.height, {
      threshold: 0.16,
      includeAA: false,
    });
    const ratio = different / (actual.width * actual.height);
    if (ratio > 0.002) {
      await mkdir(visualDiffDir, { recursive: true });
      await writeFile(join(visualDiffDir, name.replace(".png", ".actual.png")), actualBuffer);
      await writeFile(join(visualDiffDir, name.replace(".png", ".diff.png")), PNG.sync.write(diff));
      throw new Error(`visual regression ${name}: ${(ratio * 100).toFixed(2)}% pixels changed`);
    }
  };
}

export async function assertShellRegionsDoNotOverlap(page) {
  const result = await page.evaluate(() => {
    const rect = (selector) => document.querySelector(selector)?.getBoundingClientRect() ?? null;
    const overlaps = (a, b) => Boolean(a && b && a.left < b.right - 1 && a.right > b.left + 1 && a.top < b.bottom - 1 && a.bottom > b.top + 1);
    const chrome = rect(".chrome");
    const body = rect(".body");
    const side = rect(".side");
    const workspace = rect(".workspace");
    const boardMain = rect(".board-main");
    const detailNode = document.querySelector(".board-shell > .issue-detail, .lifted-run > .issue-detail");
    const detail = detailNode?.getBoundingClientRect() ?? null;
    const floatingDetail = detailNode ? getComputedStyle(detailNode).position === "absolute" : false;
    const mobileNav = rect(".mobile-nav");
    const lanes = [...document.querySelectorAll(".lane")].map((node) => node.getBoundingClientRect());
    const laneOverlap = lanes.some((lane, index) => lanes.slice(index + 1).some((other) => overlaps(lane, other)));
    return {
      chromeOverBody: overlaps(chrome, body),
      sideOverWorkspace: overlaps(side, workspace),
      unexpectedBoardDetailOverlap: overlaps(boardMain, detail) && !floatingDetail,
      bodyOverMobileNav: overlaps(body, mobileNav),
      laneOverlap,
      horizontalOverflow: document.documentElement.scrollWidth - document.documentElement.clientWidth,
      verticalOverflow: document.documentElement.scrollHeight - document.documentElement.clientHeight,
    };
  });
  if (Object.values(result).some((value) => value === true)) {
    throw new Error(`shell regions overlap: ${JSON.stringify(result)}`);
  }
  if (result.horizontalOverflow > 0 || result.verticalOverflow > 0) {
    throw new Error(`shell created page-level overflow: ${JSON.stringify(result)}`);
  }
}
