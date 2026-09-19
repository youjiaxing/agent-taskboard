import { readdir, readFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const desktopDir = join(dirname(fileURLToPath(import.meta.url)), "..");
const sourceDir = join(desktopDir, "src");
const e2eDir = join(desktopDir, "e2e");
const failures = [];

const readSource = (path) => readFile(join(sourceDir, path), "utf8");
const cssNames = (await readdir(sourceDir)).filter((name) => name.endsWith(".css")).sort();
const cssFiles = await Promise.all(cssNames.map(async (name) => [name, await readSource(name)]));

for (const [name, css] of cssFiles) {
  const lines = css.split("\n");
  lines.forEach((line, index) => {
    if (/(?:#[0-9a-f]{3,8}\b|rgba?\(|hsla?\()/i.test(line) && !/^\s*--[\w-]+\s*:/.test(line)) {
      failures.push(`${name}:${index + 1} hard-codes a color outside a token declaration`);
    }
  });
  const declaration = /(^|[;{]\s*)(margin(?:-[a-z]+)?|padding(?:-[a-z]+)?|gap|row-gap|column-gap|border-radius|z-index)\s*:\s*([^;}]+)/gm;
  for (const match of css.matchAll(declaration)) {
    const [, , property, rawValue] = match;
    const value = rawValue.trim();
    const line = css.slice(0, match.index).split("\n").length;
    if (property === "z-index") {
      if (!/^var\(--layer-[\w-]+\)$/.test(value)) failures.push(`${name}:${line} bypasses layer tokens with ${property}: ${value}`);
      continue;
    }
    const literals = [...value.matchAll(/-?(?:\d*\.)?\d+(?:px|rem|em)\b/g)].map((item) => item[0]);
    if (literals.some((literal) => Number.parseFloat(literal) !== 0)) {
      failures.push(`${name}:${line} bypasses spacing/radius tokens with ${property}: ${value}`);
    }
  }
}

const primitivePaths = [
  "components/primitives.ts",
  "components/dialog.ts",
  "components/dialog-controller.ts",
];
for (const path of primitivePaths) {
  const source = await readSource(path);
  if (/\b(?:Issue|Run)\b/.test(source)) failures.push(`${path} makes an Issue/Run decision in an interface primitive`);
}

const domainComponentPaths = ["components/issue.ts", "components/refresh-status.ts"];
for (const path of domainComponentPaths) {
  const source = await readSource(path);
  if (/from\s+["'][^"']*(?:\/|\.)ui["']|from\s+["'][^"']*(?:\/|\.)rpc["']/.test(source)) {
    failures.push(`${path} imports global ui or RPC`);
  }
  if (/\bSnapshot\b/.test(source)) failures.push(`${path} accepts the full Snapshot`);
  if (/\brpc\s*\(/.test(source)) failures.push(`${path} calls RPC directly`);
}

const productionEntries = await collectFiles(sourceDir);
const productionSource = (await Promise.all(productionEntries.map(async (path) => [path, await readFile(path, "utf8")])));
const forbiddenProductionPatterns = [
  [/(?:warm-paper|plain-paper|plain-night|lastLightTheme|\bsetTheme\b)/, "legacy theme key"],
  [/960px/, "legacy 960px branch"],
  [/\.overlay\b/, "legacy overlay shell"],
  [/data-(?:floating|workbench-panel|panel-drag|panel-mode)/, "free-floating panel marker"],
  [/--panel-(?:x|y|height)\b|agent-taskboard-panel-layout/, "free-coordinate panel persistence"],
  [/\blegacyGraph\b|data-field=["']closedContext["']/, "legacy dependency-graph path"],
  [/\bpanelToggle\b|detail-panel-toggle/, "duplicate Issue-panel toggle path"],
  [/close-settings|close-changes|--inspector-panel-width/, "retired duplicate control or dead panel value"],
];
for (const [path, source] of productionSource) {
  for (const [pattern, label] of forbiddenProductionPatterns) {
    if (pattern.test(source)) failures.push(`${relative(path)} still contains ${label}`);
  }
}

const viewHelpers = await readSource("view-helpers.ts");
const mobileBreakpoint = Number(viewHelpers.match(/MOBILE_BREAKPOINT\s*=\s*([\d.]+)/)?.[1]);
const desktopBreakpoint = Number(viewHelpers.match(/FULL_DESKTOP_BREAKPOINT\s*=\s*([\d.]+)/)?.[1]);
const allCss = cssFiles.map(([, source]) => source).join("\n");
if (mobileBreakpoint !== 640 || desktopBreakpoint !== 900) {
  failures.push(`JavaScript breakpoints must stay 640/900, got ${mobileBreakpoint}/${desktopBreakpoint}`);
}
for (const expected of [
  `(max-width: ${(mobileBreakpoint - 0.02).toFixed(2)}px)`,
  `(min-width: ${mobileBreakpoint}px)`,
  `(max-width: ${(desktopBreakpoint - 0.02).toFixed(2)}px)`,
]) {
  if (!allCss.includes(expected)) failures.push(`CSS is missing the JavaScript breakpoint peer ${expected}`);
}
const allowedCssBreakpoints = new Set([639.98, 640, 899.98]);
for (const [name, css] of cssFiles) {
  const mediaWidths = [...css.matchAll(/@media[^\n{]*?(?:min|max)-width:\s*([\d.]+)px/g)]
    .map((match) => Number(match[1]));
  for (const width of mediaWidths) {
    if (width === 600 && name === "shell-dialogs.css") continue;
    if (!allowedCssBreakpoints.has(width)) {
      failures.push(`${name} introduces an uncontracted breakpoint at ${width}px`);
    }
  }
}

const appRenderer = await readSource("render/app.ts");
for (const action of ["right-rail", "changes", "appearance", "settings", "more"]) {
  const count = appRenderer.match(new RegExp(`data-global-action=["']${action}["']`, "g"))?.length ?? 0;
  if (count !== 1) failures.push(`global action ${action} must have exactly one renderer, got ${count}`);
}
const shellRenderer = await readSource("render/shell.ts");
if (!/<aside class="fixed-changes-panel" data-fixed-panel="changes-panel">/.test(shellRenderer)) {
  failures.push("view changes must render as the fixed right-side panel");
}
const viewChangesStart = shellRenderer.indexOf("export function viewChangesPanel");
const viewChangesEnd = shellRenderer.indexOf("export function changeRepoBlock", viewChangesStart);
const viewChangesSource = shellRenderer.slice(viewChangesStart, viewChangesEnd);
if (viewChangesSource.includes("data-dialog") || viewChangesSource.includes("dialog(")) {
  failures.push("view changes must not render through the dialog system");
}

const visualRegression = await readFile(join(e2eDir, "visual-regression.mjs"), "utf8");
const expectedVisualNames = [
  "desktop-main-light.png",
  "desktop-main-dark.png",
  "focus-workspace-rail.png",
  "settings-appearance.png",
  "appearance-menu.png",
  "mobile-focus-workspace.png",
];
const declaredVisualNames = [...visualRegression.matchAll(/^\s*"([\w-]+\.png)",$/gm)].map((match) => match[1]);
if (JSON.stringify(declaredVisualNames) !== JSON.stringify(expectedVisualNames)) {
  failures.push(`long-term visual state declaration drifted: ${JSON.stringify(declaredVisualNames)}`);
}
const e2eEntries = await collectFiles(e2eDir, (path) => path.endsWith(".mjs"));
const visualCalls = [];
for (const path of e2eEntries) {
  const source = await readFile(path, "utf8");
  for (const match of source.matchAll(/\.assertVisual\("([\w-]+\.png)"\)/g)) visualCalls.push(match[1]);
  if (/issue-99-[^"'\s]*\.png/.test(source)) failures.push(`${relative(path)} references an issue-99 visual baseline`);
}
if (JSON.stringify([...visualCalls].sort()) !== JSON.stringify([...expectedVisualNames].sort())) {
  failures.push(`visual journeys must call exactly the six long-term states: ${JSON.stringify(visualCalls)}`);
}
const baselineDir = join(e2eDir, "baselines");
let baselineNames = [];
try {
  baselineNames = (await readdir(baselineDir)).filter((name) => name.endsWith(".png")).sort();
} catch (error) {
  if (error.code !== "ENOENT") throw error;
}
if (baselineNames.length && JSON.stringify(baselineNames) !== JSON.stringify([...expectedVisualNames].sort())) {
  failures.push(`visual baselines must be empty before approval or contain exactly the six approved states: ${JSON.stringify(baselineNames)}`);
}

if (failures.length) {
  console.error(`UI source contracts failed:\n- ${failures.join("\n- ")}`);
  process.exit(1);
}
console.log(`UI source contracts ok (${cssFiles.length} stylesheets, ${productionEntries.length} source files, six visual states)`);

async function collectFiles(directory, predicate = () => true) {
  const result = [];
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) result.push(...await collectFiles(path, predicate));
    else if (predicate(path)) result.push(path);
  }
  return result.sort();
}

function relative(path) {
  return path.slice(desktopDir.length + 1);
}
