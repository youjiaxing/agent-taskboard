#!/usr/bin/env node

import { readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const VERSION = /^(\d+)\.(\d+)\.(\d+)(?:-[0-9A-Za-z.-]+)?$/;

export function compareVersions(left, right) {
  const a = Array.isArray(left) ? left : parseStableVersion(left);
  const b = Array.isArray(right) ? right : parseStableVersion(right);
  for (let index = 0; index < a.length; index += 1) {
    if (a[index] !== b[index]) return Math.sign(a[index] - b[index]);
  }
  return 0;
}

export function nextAutomaticVersion(current, latestTag = "", latestReleaseComplete = true) {
  const currentVersion = parseStableVersion(current);
  const latestVersion = latestTag ? parseStableVersion(latestTag.replace(/^v/, "")) : null;
  if (latestVersion) {
    const comparison = compareVersions(currentVersion, latestVersion);
    if (comparison > 0) return formatVersion(currentVersion);
    if (comparison === 0 && !latestReleaseComplete) return formatVersion(currentVersion);
    return incrementPatch(latestVersion);
  }
  return incrementPatch(currentVersion);
}

export async function updateReleaseVersions(repoRoot, version) {
  validateVersion(version);
  const edits = [
    ["apps/desktop/src-tauri/tauri.conf.json", replaceJsonVersion],
    ["apps/desktop/package.json", replaceJsonVersion],
    ["apps/desktop/src-tauri/Cargo.toml", replaceCargoVersion],
    ["crates/host-kernel/Cargo.toml", replaceCargoVersion],
    ["apps/desktop/package-lock.json", replaceNpmLockVersion],
    ["Cargo.lock", replaceCargoLockVersion],
  ];
  const changed = [];

  for (const [relativePath, replace] of edits) {
    const filePath = path.join(repoRoot, relativePath);
    const before = await readFile(filePath, "utf8");
    const after = replace(before, version);
    if (after === before) {
      throw new Error(`${relativePath}: no version field was updated`);
    }
    await writeFile(filePath, after);
    changed.push(relativePath);
  }

  return changed;
}

function parseStableVersion(value) {
  const normalized = String(value).replace(/^v/, "").split("-", 1)[0];
  if (!VERSION.test(normalized)) {
    throw new Error(`expected a stable semantic version, received ${JSON.stringify(value)}`);
  }
  return normalized.split(".").map(Number);
}

function validateVersion(value) {
  parseStableVersion(value);
}

function replaceJsonVersion(text, version) {
  return text.replace(/("version"\s*:\s*")[^"]+(")/, `$1${version}$2`);
}

function replaceCargoVersion(text, version) {
  return text.replace(/^(version\s*=\s*")[^"]+(")/m, `$1${version}$2`);
}

function replaceNpmLockVersion(text, version) {
  const lock = JSON.parse(text);
  lock.version = version;
  if (lock.packages?.[""]) {
    lock.packages[""].version = version;
  }
  return `${JSON.stringify(lock, null, 2)}\n`;
}

function replaceCargoLockVersion(text, version) {
  let replacements = 0;
  const updated = text.replace(
    /(\[\[package\]\]\nname = "(?:agent-taskboard|host-kernel)"\nversion = ")[^"]+(")/g,
    (_match, prefix, suffix) => {
      replacements += 1;
      return `${prefix}${version}${suffix}`;
    },
  );
  if (replacements !== 2) {
    throw new Error(`Cargo.lock: expected 2 local package versions, updated ${replacements}`);
  }
  return updated;
}

function formatVersion([major, minor, patch]) {
  return `${major}.${minor}.${patch}`;
}

function incrementPatch(version) {
  return formatVersion([version[0], version[1], version[2] + 1]);
}

if (path.basename(process.argv[1] ?? "") === path.basename(fileURLToPath(import.meta.url))) {
  const command = process.argv[2];
  if (command === "--next") {
    const [, current, latestTag = "", latestReleaseComplete = "true"] = process.argv.slice(2);
    console.log(nextAutomaticVersion(current, latestTag, latestReleaseComplete !== "false"));
  } else {
    const version = command;
    if (!version || process.argv.includes("--help")) {
      console.error("usage: node auto-release-version.mjs <version>");
      process.exit(1);
    }
    const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../..");
    for (const relativePath of await updateReleaseVersions(repoRoot, version)) {
      console.log(`${relativePath}: updated to ${version}`);
    }
  }
}
