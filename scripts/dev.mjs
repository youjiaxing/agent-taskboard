import { execFileSync, spawn } from "node:child_process";

const isWindows = process.platform === "win32";
const npm = isWindows ? "npm.cmd" : "npm";
const child = spawn(npm, ["--prefix", "apps/desktop", "run", "tauri", "dev"], {
  detached: !isWindows,
  env: process.env,
  stdio: "inherit",
});

let stopping = false;
let forceStopTimer;
let exitTimer;

function descendantPids(rootPid) {
  if (isWindows) {
    return [];
  }

  let rows;
  try {
    rows = execFileSync("ps", ["-axo", "pid=,ppid="], { encoding: "utf8" });
  } catch {
    return [];
  }

  const childrenByParent = new Map();
  for (const row of rows.trim().split("\n")) {
    const [pid, ppid] = row.trim().split(/\s+/).map(Number);
    if (!pid || !ppid) {
      continue;
    }
    const children = childrenByParent.get(ppid) ?? [];
    children.push(pid);
    childrenByParent.set(ppid, children);
  }

  const descendants = [];
  const pending = [rootPid];
  while (pending.length > 0) {
    const parent = pending.pop();
    for (const pid of childrenByParent.get(parent) ?? []) {
      descendants.push(pid);
      pending.push(pid);
    }
  }
  return descendants;
}

function signalDevTree(signal) {
  if (!child.pid) {
    return;
  }

  if (!isWindows) {
    try {
      process.kill(-child.pid, signal);
    } catch (error) {
      if (error.code !== "ESRCH") {
        throw error;
      }
    }
  }

  for (const pid of descendantPids(child.pid).reverse()) {
    try {
      process.kill(pid, signal);
    } catch (error) {
      if (error.code !== "ESRCH") {
        throw error;
      }
    }
  }
}

function stop(signal) {
  if (stopping) {
    signalDevTree("SIGKILL");
    return;
  }
  stopping = true;
  signalDevTree(signal);
  forceStopTimer = setTimeout(() => signalDevTree("SIGKILL"), 3000);
  exitTimer = setTimeout(() => process.exit(128 + (signal === "SIGTERM" ? 15 : 2)), 4000);
  forceStopTimer.unref();
  exitTimer.unref();
}

process.once("SIGINT", () => stop("SIGINT"));
process.once("SIGTERM", () => stop("SIGTERM"));

child.once("error", (error) => {
  console.error(error);
  process.exit(1);
});

child.once("exit", (code, signal) => {
  if (forceStopTimer) {
    clearTimeout(forceStopTimer);
  }
  if (exitTimer) {
    clearTimeout(exitTimer);
  }
  process.exit(signal ? 128 + ({ SIGINT: 2, SIGTERM: 15 }[signal] ?? 1) : (code ?? 1));
});
