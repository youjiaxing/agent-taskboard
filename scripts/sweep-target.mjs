#!/usr/bin/env node
// 控制仓库 target/ 的体积：先删增量缓存，超过阈值再整目录清理。
//
// Rust 的 target/ 会随每次改编译参数、换工具链、跑不同构建而累积陈旧产物，
// 本仓库实测曾达 24GB；整目录清理后冷编译约 41 秒，所以按阈值清理很划算。
// 既作为模块供 scripts/dev.mjs 在启动前调用，也作为 CLI（npm run sweep）手动使用。

import { execFileSync } from "node:child_process";
import { existsSync, rmSync } from "node:fs";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const MIN_LIMIT_GB = 1;
const CACHE_DIRS = ["target/debug/incremental", "target/release/incremental"];
const REPO_ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

export const DEFAULT_LIMIT_GB = 6;

export function sweepTarget({ limitGb = DEFAULT_LIMIT_GB, dryRun = false, force = false, log = console.log } = {}) {
  assertLimit(limitGb);
  const target = path.join(REPO_ROOT, "target");
  if (!existsSync(target)) {
    log("target/ 不存在，无需清理");
    return { beforeGb: 0, afterGb: 0, removed: [], cleanedWholeTarget: false };
  }

  const beforeGb = sizeGb(target);
  const removed = [];
  for (const relative of CACHE_DIRS) {
    const directory = path.join(REPO_ROOT, relative);
    if (!existsSync(directory)) {
      continue;
    }
    const freedGb = sizeGb(directory);
    removed.push(relative);
    if (dryRun) {
      log(`[预演] 会删除 ${relative}（${freedGb.toFixed(1)}GB）`);
      continue;
    }
    rmSync(directory, { recursive: true, force: true });
  }

  if (dryRun) {
    const afterCaches = sizeGb(target);
    const wouldClean = afterCaches > limitGb || force;
    log(`[预演] target/ ${afterCaches.toFixed(1)}GB / 上限 ${limitGb}GB —— ${wouldClean ? "会执行 cargo clean" : "未超上限，保留依赖产物"}`);
    return { beforeGb, afterGb: afterCaches, removed, cleanedWholeTarget: false };
  }

  const afterCaches = sizeGb(target);
  if (afterCaches > limitGb || force) {
    log(`target/ ${afterCaches.toFixed(1)}GB 超过上限 ${limitGb}GB，执行 cargo clean`);
    execFileSync(cargoPath(), ["clean"], { cwd: REPO_ROOT, stdio: "inherit" });
    log("target/ 已清空，下次构建为冷编译（本仓库约 41 秒）");
    return { beforeGb, afterGb: 0, removed, cleanedWholeTarget: true };
  }

  log(`target/ ${afterCaches.toFixed(1)}GB / 上限 ${limitGb}GB —— 未超上限，仅清理增量缓存`);
  return { beforeGb, afterGb: afterCaches, removed, cleanedWholeTarget: false };
}

function assertLimit(limitGb) {
  if (!Number.isFinite(limitGb) || limitGb < MIN_LIMIT_GB) {
    throw new Error(`阈值需要一个不小于 ${MIN_LIMIT_GB} 的 GB 数字，收到 ${JSON.stringify(limitGb)}`);
  }
}

function sizeGb(directory) {
  let output;
  try {
    output = execFileSync("du", ["-sk", directory], { encoding: "utf8", stdio: ["ignore", "pipe", "ignore"] });
  } catch {
    throw new Error(`无法读取 ${directory} 的体积`);
  }
  return Number(output.split(/\s+/)[0]) / 1024 / 1024;
}

function cargoPath() {
  const local = path.join(process.env.HOME ?? "", ".cargo/bin/cargo");
  return existsSync(local) ? local : "cargo";
}

function runCli() {
  const args = process.argv.slice(2);
  const option = (name, fallback) => {
    const index = args.indexOf(`--${name}`);
    return index === -1 ? fallback : args[index + 1];
  };

  const rawLimit = option("limit", null);
  const limitGb = rawLimit === null ? DEFAULT_LIMIT_GB : Number(rawLimit);
  if (rawLimit !== null && !Number.isFinite(limitGb)) {
    console.error(`--limit 需要数字，收到 ${JSON.stringify(rawLimit)}`);
    process.exit(1);
  }

  try {
    sweepTarget({
      limitGb,
      dryRun: args.includes("--dry-run"),
      force: args.includes("--force"),
    });
  } catch (error) {
    console.error(error.message);
    process.exit(1);
  }
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  runCli();
}
