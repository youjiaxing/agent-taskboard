import { existsSync } from "node:fs";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import pixelmatch from "pixelmatch";
import { PNG } from "pngjs";

const defaultVisualBaselineDir = join(dirname(fileURLToPath(import.meta.url)), "baselines");
const visualBaselineDir = process.env.VISUAL_BASELINE_DIR
  ? resolve(process.env.VISUAL_BASELINE_DIR)
  : defaultVisualBaselineDir;
const visualDiffDir = process.env.VISUAL_DIFF_DIR ?? join("target", "visual-diffs");
const updateVisualBaselines = process.env.UPDATE_VISUAL_BASELINES === "1";
const visualBaselinesEnabled = updateVisualBaselines || existsSync(visualBaselineDir);
const maxVisualDiffRatio = Number(process.env.VISUAL_DIFF_MAX_RATIO ?? "0.002");
const deterministicNowMs = 1_787_748_507_000;

export const VISUAL_BASELINE_NAMES = Object.freeze([
  "desktop-main-light.png",
  "desktop-main-dark.png",
  "focus-workspace-rail.png",
  "settings-appearance.png",
  "appearance-menu.png",
  "mobile-focus-workspace.png",
]);

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
    if (!VISUAL_BASELINE_NAMES.includes(name)) {
      throw new Error(`unexpected long-term visual state ${name}`);
    }
    if (!visualBaselinesEnabled) return;
    await page.evaluate(async () => {
      if (document.fonts) await document.fonts.ready;
    });
    const actualBuffer = await page.screenshot({
      fullPage: false,
      animations: "disabled",
      caret: "hide",
    });
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
    if (ratio > maxVisualDiffRatio) {
      await mkdir(visualDiffDir, { recursive: true });
      await writeFile(join(visualDiffDir, name.replace(".png", ".actual.png")), actualBuffer);
      await writeFile(join(visualDiffDir, name.replace(".png", ".diff.png")), PNG.sync.write(diff));
      throw new Error(`visual regression ${name}: ${(ratio * 100).toFixed(2)}% pixels changed`);
    }
  };
}

export async function assertNecessaryTextContrast(page, label) {
  await page.waitForTimeout(260);
  await page.evaluate(async () => {
    const animations = document.getAnimations({ subtree: true }).filter((animation) => {
      const iterations = animation.effect?.getComputedTiming().iterations;
      return iterations !== Infinity;
    });
    await Promise.race([
      Promise.all(animations.map((animation) => animation.finished.catch(() => undefined))),
      new Promise((resolve) => setTimeout(resolve, 500)),
    ]);
  });
  const failures = await page.evaluate(() => {
    const parseColor = (value) => {
      const match = value.match(/^rgba?\(\s*([\d.]+)[, ]+([\d.]+)[, ]+([\d.]+)(?:\s*[,/]\s*([\d.]+%?))?\s*\)$/i);
      if (!match) return null;
      const alpha = match[4]?.endsWith("%") ? Number.parseFloat(match[4]) / 100 : Number.parseFloat(match[4] ?? "1");
      return { r: Number(match[1]), g: Number(match[2]), b: Number(match[3]), a: alpha };
    };
    const composite = (front, back, alpha = front.a) => ({
      r: front.r * alpha + back.r * (1 - alpha),
      g: front.g * alpha + back.g * (1 - alpha),
      b: front.b * alpha + back.b * (1 - alpha),
      a: 1,
    });
    const backgroundBehind = (start) => {
      const layers = [];
      for (let current = start; current; current = current.parentElement) {
        const color = parseColor(getComputedStyle(current).backgroundColor);
        if (color && color.a > 0) layers.push(color);
      }
      return layers.reverse().reduce((back, front) => composite(front, back), { r: 255, g: 255, b: 255, a: 1 });
    };
    const linear = (channel) => {
      const value = channel / 255;
      return value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
    };
    const luminance = (color) => linear(color.r) * 0.2126 + linear(color.g) * 0.7152 + linear(color.b) * 0.0722;
    const contrast = (left, right) => {
      const values = [luminance(left), luminance(right)].sort((a, b) => b - a);
      return (values[0] + 0.05) / (values[1] + 0.05);
    };
    const selector = (node) => {
      const parts = [];
      for (let current = node; current && parts.length < 4; current = current.parentElement) {
        const name = current.tagName.toLowerCase();
        const id = current.id ? `#${current.id}` : "";
        const classes = [...current.classList].slice(0, 2).map((item) => `.${item}`).join("");
        parts.unshift(`${name}${id}${classes}`);
      }
      return parts.join(" > ");
    };
    const nodes = [...document.querySelectorAll("body *")].filter((node) => {
      if (!(node instanceof HTMLElement)) return false;
      if (node.closest("[aria-hidden='true'], .sr-only, .xterm")) return false;
      if (node.closest("button:disabled, [aria-disabled='true']")) return false;
      const style = getComputedStyle(node);
      const rect = node.getBoundingClientRect();
      if (style.display === "none" || style.visibility === "hidden" || rect.width === 0 || rect.height === 0) return false;
      return [...node.childNodes].some((child) => child.nodeType === Node.TEXT_NODE && child.textContent?.trim());
    });
    return nodes.flatMap((node) => {
      const foreground = parseColor(getComputedStyle(node).color);
      if (!foreground) return [];
      let opacity = 1;
      let backgroundOwner = null;
      let localBackground = null;
      for (let current = node; current; current = current.parentElement) {
        const style = getComputedStyle(current);
        opacity *= Number.parseFloat(style.opacity || "1");
        const color = parseColor(style.backgroundColor);
        if (color && color.a > 0) {
          backgroundOwner = current;
          localBackground = color;
          break;
        }
      }
      if (!backgroundOwner || !localBackground) return [];
      const outerBackground = backgroundBehind(backgroundOwner.parentElement);
      const opaqueLocalBackground = composite(localBackground, outerBackground);
      const renderedBackground = composite(opaqueLocalBackground, outerBackground, opacity);
      const localForeground = composite(foreground, opaqueLocalBackground);
      const renderedForeground = composite(localForeground, outerBackground, opacity);
      const ratio = contrast(renderedForeground, renderedBackground);
      return ratio < 4.5
        ? [{ selector: selector(node), text: node.textContent.trim().slice(0, 80), ratio: Number(ratio.toFixed(2)), opacity: Number(opacity.toFixed(3)) }]
        : [];
    });
  });
  if (failures.length) {
    throw new Error(`${label} necessary text contrast below 4.5:1: ${JSON.stringify(failures.slice(0, 20))}`);
  }
}

export async function assertShellRegionsDoNotOverlap(page) {
  const result = await page.evaluate(() => {
    const rect = (selector) => {
      const node = document.querySelector(selector);
      if (!node || !node.getClientRects().length) return null;
      return node.getBoundingClientRect();
    };
    const overlaps = (a, b) => Boolean(a && b && a.left < b.right - 1 && a.right > b.left + 1 && a.top < b.bottom - 1 && a.bottom > b.top + 1);
    const chrome = rect(".chrome");
    const body = rect(".body");
    const side = rect(".side");
    const workspace = rect(".workspace");
    const contentToolbar = rect("[data-page-toolbar]");
    const pageContent = rect(".lanes, .dep-graph, .focus-workspace-layout, .settings-content, .overview-controls, .usage-ranges");
    const focusMain = rect(".focus-workspace-main");
    const focusSide = rect(".fixed-right-rail, .fixed-changes-panel");
    const mobilePanel = rect(".mobile-workspace-panel, .mobile-board-view");
    const mobileNav = rect(".mobile-nav");
    const mobileInput = rect(".mobile-input-row");
    const dialogPanel = rect("[data-dialog-root='true'] [role='dialog']");
    return {
      chromeOverBody: overlaps(chrome, body),
      sideOverWorkspace: overlaps(side, workspace),
      toolbarOverContent: overlaps(contentToolbar, pageContent),
      focusMainOverSide: overlaps(focusMain, focusSide),
      mobilePanelOverNav: overlaps(mobilePanel, mobileNav),
      mobilePanelOverInput: overlaps(mobilePanel, mobileInput),
      mobileNavOverInput: overlaps(mobileNav, mobileInput),
      dialogOutsideViewport: Boolean(dialogPanel && (dialogPanel.left < -1 || dialogPanel.top < -1 || dialogPanel.right > innerWidth + 1 || dialogPanel.bottom > innerHeight + 1)),
      toolbarRect: contentToolbar ? [contentToolbar.left, contentToolbar.top, contentToolbar.right, contentToolbar.bottom] : null,
      contentRect: pageContent ? [pageContent.left, pageContent.top, pageContent.right, pageContent.bottom] : null,
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
