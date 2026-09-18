#!/usr/bin/env node
// 控制仓库 target/ 的体积：先删增量缓存，超过阈值再整目录清理。
//
// Rust 的 target/ 会随每次改编译参数、换工具链、跑不同构建而累积陈旧产物，
// 本仓库实测曾达 24GB。整目录清理后冷编译约 50 秒，所以按阈值清理很划算。

import { execFileSync } from "node:child_process";
import { existsSync, rmSync, statSync } from "node:fs";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const MIN_LIMIT_GB = 1;
const DEFAULT_LIMIT_GB = 10;
const CACHE_DIRS = ["target/debug/incremental", "target/release/incremental"];

const args = process.argv.slice(2);

function option(name, fallback) {
  const index = args.indexOf(`--${name}`);
  return index === -1 ? fallback : args[index + 1];
}

const dryRun = args.includes("--dry-run");
const force = args.includes("--force");
const limitGb = Number(option("limit", DEFAULT_LIMIT_GB));

if (!Number.isFinite(limitGb) || limitGb < MIN_LIMIT_GB) {
  console.error(`--limit 需要一个不小于 ${MIN_LIMIT_GB} 的 GB 数字，收到 ${JSON.stringify(option("limit", ""))}`);
  process.exit(1);
}

const repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const target = path.join(repo, "target");

if (!existsSync(target)) {
  console.log("target/ 不存在，无需处理");
  process.exit(0);
}

const sizeGb = (directory) => {
  const output = execFileSync("du", ["-sk", directory], { encoding: "utf8" });
  return Number(output.split(/\s+/)[0]) / 1024 / 1024;
};

const before = sizeGb(target);
console.log(`target/ 当前 ${before.toFixed(1)}GB（阈值 ${limitGb}GB${force ? "，--force" : ""}）`);

for (const relative of CACHE_DIRS) {
  const directory = path.join(repo, relative);
  if (!existsSync(directory)) continue;
  const freed = sizeGb(directory);
  if (dryRun) {
    console.log(`[预演] 会删除 ${relative}（${freed.toFixed(1)}GB）`);
    continue;
  }
  rmSync(directory, { recursive: true, force: true });
  console.log(`已删除 ${relative}（${freed.toFixed(1)}GB）`);
}

if (dryRun) {
  const afterCaches = existsSync(target) ? sizeGb(target) : 0;
  console.log(`[预演] 删缓存后约 ${afterCaches.toFixed(1)}GB；${afterCaches > limitGb || force ? "还会执行 cargo clean" : "未超阈值，保留依赖产物"}`);
  process.exit(0);
}

const afterCaches = sizeGb(target);
if (afterCaches > limitGb || force) {
  console.log(`仍为 ${afterCaches.toFixed(1)}GB，执行 cargo clean`);
  execFileSync(cargoPath(), ["clean"], { cwd: repo, stdio: "inherit" });
  console.log("target/ 已清空，下次构建为冷编译（本仓库约 50 秒）");
} else {
  console.log(`留在阈值内，保留依赖产物，未做整目录清理（现在 ${afterCaches.toFixed(1)}GB）`);
}

function cargoPath() {
  const local = path.join(process.env.HOME ?? "", ".cargo/bin/cargo");
  return existsSync(local) ? local : "cargo";
}
