import { createHash } from "node:crypto";
import { lstat, mkdir, readFile, readdir, readlink, writeFile } from "node:fs/promises";
import path from "node:path";

export const DEFAULT_INPUTS = [
  "Cargo.lock",
  "Cargo.toml",
  "crates/host-kernel",
  "apps/desktop/index.html",
  "apps/desktop/package-lock.json",
  "apps/desktop/package.json",
  "apps/desktop/tsconfig.json",
  "apps/desktop/vite.config.ts",
  "apps/desktop/scripts",
  "apps/desktop/src",
  "apps/desktop/src-tauri",
  "apps/desktop/e2e/baselines/appearance-menu.png",
  "apps/desktop/e2e/baselines/desktop-main-dark.png",
  "apps/desktop/e2e/baselines/desktop-main-light.png",
  "apps/desktop/e2e/baselines/focus-workspace-rail.png",
  "apps/desktop/e2e/baselines/mobile-focus-workspace.png",
  "apps/desktop/e2e/baselines/settings-appearance.png",
];

export const DEFAULT_EXCLUSIONS = [
  "apps/desktop/src-tauri/acceptance",
  "apps/desktop/src-tauri/gen",
  "apps/desktop/src-tauri/target",
  "crates/host-kernel/tests",
];

const EXCLUDED_DIRECTORY_NAMES = new Set([
  ".git",
  "dist",
  "logs",
  "node_modules",
  "target",
  "temp",
  "tmp",
]);
const EXCLUDED_FILE_NAMES = new Set([".DS_Store"]);
const EXCLUDED_FILE_SUFFIXES = [".log", ".swp", ".temp", ".tmp"];


export async function collectManifest(
  root,
  inputs = DEFAULT_INPUTS,
  exclusions = DEFAULT_EXCLUSIONS,
) {
  const files = new Map();
  for (const input of inputs) {
    await collectPath(root, normalizePath(input), files, exclusions);
  }
  return {
    version: 1,
    files: [...files.values()].sort((left, right) => comparePaths(left.path, right.path)),
  };
}

export function canonicalManifest(manifest) {
  const normalized = {
    version: manifest.version,
    files: manifest.files.map(({ path: filePath, mode, sha256 }) => ({
      path: filePath,
      mode,
      sha256,
    })),
  };
  return `${JSON.stringify(normalized, null, 2)}\n`;
}

export function manifestDigest(manifest) {
  return sha256(Buffer.from(canonicalManifest(manifest)));
}

export async function writeManifest(outputDir, manifest) {
  const content = canonicalManifest(manifest);
  const digest = sha256(Buffer.from(content));
  await mkdir(outputDir, { recursive: true });
  await writeFile(path.join(outputDir, "manifest.json"), content);
  await writeFile(path.join(outputDir, "manifest.sha256"), `${digest}\n`);
  return digest;
}

export function compareManifests(expected, current) {
  const expectedByPath = new Map(expected.files.map((file) => [file.path, file]));
  const currentByPath = new Map(current.files.map((file) => [file.path, file]));
  const expectedPaths = [...expectedByPath.keys()].sort(comparePaths);
  const currentPaths = [...currentByPath.keys()].sort(comparePaths);
  const differences = [];

  for (const filePath of expectedPaths) {
    if (!currentByPath.has(filePath)) differences.push(`removed: ${filePath}`);
  }
  for (const filePath of currentPaths) {
    if (!expectedByPath.has(filePath)) differences.push(`added: ${filePath}`);
  }
  for (const filePath of expectedPaths) {
    const before = expectedByPath.get(filePath);
    const after = currentByPath.get(filePath);
    if (!after) continue;
    if (before.sha256 !== after.sha256) differences.push(`changed content: ${filePath}`);
    if (before.mode !== after.mode) {
      differences.push(`changed mode: ${filePath} (${before.mode} -> ${after.mode})`);
    }
  }
  return differences;
}

async function collectPath(root, relativePath, files, exclusions) {
  if (isExcluded(relativePath, exclusions)) return;
  const absolutePath = path.join(root, relativePath);
  let stats;
  try {
    stats = await lstat(absolutePath);
  } catch (error) {
    if (error?.code === "ENOENT") throw new Error(`required manifest input is missing: ${relativePath}`);
    throw error;
  }

  if (stats.isDirectory()) {
    const entries = await readdir(absolutePath);
    entries.sort(comparePaths);
    for (const entry of entries) {
      await collectPath(root, normalizePath(path.join(relativePath, entry)), files, exclusions);
    }
    return;
  }

  if (!stats.isFile() && !stats.isSymbolicLink()) {
    throw new Error(`unsupported manifest input type: ${relativePath}`);
  }
  const bytes = stats.isSymbolicLink()
    ? Buffer.from(await readlink(absolutePath))
    : await readFile(absolutePath);
  files.set(relativePath, {
    path: relativePath,
    mode: stats.isSymbolicLink() ? "120000" : stats.mode & 0o111 ? "100755" : "100644",
    sha256: sha256(bytes),
  });
}

function isExcluded(relativePath, exclusions) {
  if (
    exclusions.some(
      (prefix) => relativePath === prefix || relativePath.startsWith(`${prefix}/`),
    )
  ) {
    return true;
  }
  const parts = relativePath.split("/");
  const name = parts.at(-1) ?? "";
  return (
    parts.slice(0, -1).some((part) => EXCLUDED_DIRECTORY_NAMES.has(part)) ||
    EXCLUDED_FILE_NAMES.has(name) ||
    EXCLUDED_FILE_SUFFIXES.some((suffix) => name.endsWith(suffix))
  );
}

function normalizePath(filePath) {
  return filePath.split(path.sep).join("/");
}

function comparePaths(left, right) {
  return Buffer.compare(Buffer.from(left), Buffer.from(right));
}

function sha256(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}
