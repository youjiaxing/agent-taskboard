import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const desktop = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const bundle = path.resolve(desktop, "../../target/debug/bundle/macos/Agent Taskboard Dev.app");

if (process.platform !== "darwin") {
  console.error("dev:app 只支持 macOS：调试构建的身份标识来自 .app 包。其他平台请用 npm run dev。");
  process.exit(1);
}

const build = spawnSync(
  path.join(desktop, "node_modules/.bin/tauri"),
  ["build", "--debug", "--bundles", "app", "--config", "src-tauri/tauri.dev.conf.json"],
  { cwd: desktop, stdio: "inherit" },
);
if (build.status !== 0) {
  process.exit(build.status ?? 1);
}

if (!existsSync(bundle)) {
  console.error(`调试构建没有生成：${bundle}`);
  process.exit(1);
}

const open = spawnSync("open", [bundle], { stdio: "inherit" });
console.log(`调试构建：${bundle}`);
process.exit(open.status ?? 0);
