#!/usr/bin/env node
// 一条命令发版：同步各处版本号 → 契约自检 → 走 PR 合入 main → 打标签触发 CI。

import { execFileSync } from "node:child_process";
import { readFile, writeFile } from "node:fs/promises";
import process from "node:process";
import { fileURLToPath } from "node:url";

const SEMVER = /^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/;

const requested = process.argv[2] ?? "";
const version = requested.replace(/^v/, "");
const dryRun = process.argv.includes("--dry-run");

if (!SEMVER.test(version)) {
  fail(`用法：npm --prefix apps/desktop run release <版本号>，例如 release 0.1.2（收到 ${JSON.stringify(requested)}）`);
}

const desktop = new URL("../", import.meta.url);
const repo = new URL("../../", desktop);

const edits = [
  { path: new URL("src-tauri/tauri.conf.json", desktop), label: "tauri.conf.json", current: firstJsonVersion, apply: firstJsonVersion },
  { path: new URL("package.json", desktop), label: "package.json", current: firstJsonVersion, apply: firstJsonVersion },
  { path: new URL("src-tauri/Cargo.toml", desktop), label: "src-tauri/Cargo.toml", current: cargoPackageVersion, apply: cargoPackageVersion },
  { path: new URL("../../crates/host-kernel/Cargo.toml", desktop), label: "crates/host-kernel/Cargo.toml", current: cargoPackageVersion, apply: cargoPackageVersion },
  { path: new URL("package-lock.json", desktop), label: "package-lock.json", current: npmLockCurrent, apply: npmLockVersion },
  { path: new URL("../../Cargo.lock", desktop), label: "Cargo.lock", current: cargoLockCurrent, apply: cargoLockVersion },
];

const branch = `release/v${version}`;
const repoRoot = fileURLToPath(repo);
const relative = edits.map((edit) => fileURLToPath(edit.path).slice(repoRoot.length));

const branchState = git(["status", "--porcelain"]);
if (branchState.trim()) {
  fail("工作树不干净，先处理未提交改动再发版");
}
const current = git(["rev-parse", "--abbrev-ref", "HEAD"]).trim();
if (current !== "main") {
  fail(`发版要在 main 上进行，当前在 ${current}`);
}
if (git(["rev-parse", "--verify", "--quiet", `refs/heads/${branch}`], true)) {
  fail(`本地已存在分支 ${branch}`);
}
if (git(["rev-parse", "--verify", "--quiet", `refs/tags/v${version}`], true)) {
  fail(`标签 v${version} 已存在`);
}

for (const edit of edits) {
  const before = await readFile(edit.path, "utf8");
  const after = edit.apply(before, version);
  const previous = edit.current(before) ?? "?";
  if (after === before && previous !== version) {
    fail(`${edit.label}: 没有找到可替换的版本号`);
  }
  console.log(`${dryRun ? "[预演] " : ""}${edit.label}: ${previous} → ${version}`);
  if (!dryRun) {
    await writeFile(edit.path, after);
  }
}

console.log("\n契约自检（verify:release）");
if (dryRun) {
  console.log("[预演] 跳过：文件尚未改动");
} else {
  run(process.execPath, [fileURLToPath(new URL("scripts/verify-release.mjs", desktop))], {
    cwd: fileURLToPath(desktop),
    env: { ...process.env, RELEASE_TAG: `v${version}` },
  });
}

if (dryRun) {
  console.log(`\n[预演] 后续会：建分支 ${branch} → 提交 chore(release): ${version} → 推分支 → 开 PR → 合并 → 打标签 v${version} 并推送触发 CI`);
  process.exit(0);
}

console.log(`\n建分支 ${branch} 并提交`);
git(["checkout", "-b", branch]);
git(["add", ...relative]);
git(["commit", "-m", `chore(release): ${version}`]);
git(["push", "-u", "origin", "HEAD"]);

console.log("\n开 PR 并合并");
const prUrl = gh([
  "pr", "create",
  "--title", `chore(release): ${version}`,
  "--body", [
    "## 摘要",
    "",
    `把各处版本号同步到 ${version}，随后打标签触发三平台构建与发布。`,
    "",
    "## 范围",
    "",
    "构建工具链（版本号与 lockfile），不含产品行为。",
    "",
    "## 明确不做",
    "",
    "- 不改任何产品代码",
  ].join("\n"),
]).trim();
const prNumber = prUrl.split("/").pop();
console.log(prUrl);
gh(["pr", "merge", prNumber, "--merge", "--delete-branch"]);

console.log("\n回到 main 并打发布标签");
git(["checkout", "main"]);
git(["fetch", "origin", "main"]);
git(["merge", "--ff-only", "origin/main"]);
git(["tag", `v${version}`]);
git(["push", "origin", `v${version}`]);

console.log(`\n已推送标签 v${version}，CI 开始构建：`);
console.log("https://github.com/youjiaxing/agent-taskboard/actions/workflows/release.yml");
console.log("CI 完成后，已安装的应用下次启动会提示更新，需要人在应用里确认安装。");

function firstJsonVersion(text, next) {
  return next === undefined
    ? text.match(/"version"\s*:\s*"([^"]+)"/)?.[1]
    : text.replace(/("version"\s*:\s*")[^"]+(")/, `$1${next}$2`);
}

function cargoPackageVersion(text, next) {
  return next === undefined
    ? text.match(/^version\s*=\s*"([^"]+)"/m)?.[1]
    : text.replace(/^(version\s*=\s*")[^"]+(")/m, `$1${next}$2`);
}

function npmLockCurrent(text) {
  return JSON.parse(text).version;
}

function npmLockVersion(text, next) {
  const lock = JSON.parse(text);
  lock.version = next;
  if (lock.packages?.[""]) {
    lock.packages[""].version = next;
  }
  return `${JSON.stringify(lock, null, 2)}\n`;
}

function cargoLockCurrent(text) {
  return text.match(/\[\[package\]\]\nname = "(?:agent-taskboard|host-kernel)"\nversion = "([^"]+)"/)?.[1];
}

function cargoLockVersion(text, next) {
  return text.replace(
    /(\[\[package\]\]\nname = "(?:agent-taskboard|host-kernel)"\nversion = ")[^"]+(")/g,
    `$1${next}$2`,
  );
}

function git(args, allowFailure = false) {
  const options = {
    cwd: repoRoot,
    encoding: "utf8",
    stdio: allowFailure ? ["ignore", "pipe", "ignore"] : ["ignore", "pipe", "inherit"],
  };
  if (!allowFailure) {
    return execFileSync("git", args, options);
  }
  try {
    return execFileSync("git", args, options);
  } catch {
    return "";
  }
}

function gh(args) {
  return execFileSync("gh", args, {
    cwd: repoRoot,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "inherit"],
  });
}

function run(command, args, options) {
  execFileSync(command, args, { stdio: "inherit", ...options });
}

function fail(message) {
  console.error(message);
  process.exit(1);
}
