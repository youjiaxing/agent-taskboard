import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { test } from "node:test";
import { fileURLToPath } from "node:url";
import { createContext, runInContext } from "node:vm";
import * as marked from "marked";
import ts from "typescript";

const sourceDir = fileURLToPath(new URL("../src/", import.meta.url));

function updaterHarness(development: boolean, desktop = true, nativeDebug?: string) {
  const calls = { check: 0, download: 0, install: 0, relaunch: 0, rpc: 0 };
  const update = {
    version: "99.0.0",
    body: "",
    close: async () => {},
    download: async () => { calls.download++; },
    install: async () => { calls.install++; },
  };
  const ui = {
    snapshot: null,
    updateState: { kind: "idle" } as { kind: string; version?: string },
    pendingUpdate: null as typeof update | null,
    pendingUpdateDownloaded: false,
  };
  const gate = { allowed: true, activeRunCount: 0 };
  const localModules: Record<string, object> = {
    "ui.ts": { ui },
    "view-helpers.ts": { effectiveClientLanguage: () => "en" },
    "render/app.ts": { render: () => {} },
    "rpc.ts": { rpc: async () => {
      calls.rpc++;
      return { updateInstallGate: gate };
    } },
    "launch-picker.ts": {},
    "render/run.ts": {},
    "form-keys.ts": {},
    "workbench.ts": {},
    "render/board.ts": {},
    "shortcuts.ts": {},
    "render/run-organization.ts": {},
  };
  const locals = new Map(Object.entries(localModules).map(([path, exports]) => [
    resolve(sourceDir, path), exports,
  ]));
  const packages = new Map<string, object>([
    ["marked", marked],
    ["@tauri-apps/api/core", { isTauri: () => desktop }],
    ["@tauri-apps/plugin-updater", { check: async () => { calls.check++; return update; } }],
    ["@tauri-apps/plugin-process", { relaunch: async () => { calls.relaunch++; } }],
    ["@tauri-apps/plugin-autostart", {}],
    ["@tauri-apps/plugin-dialog", {}],
    ["@tauri-apps/plugin-notification", {}],
    ["@tauri-apps/plugin-opener", {}],
  ]);
  const context = createContext({
    window: {},
    URL,
    __importMeta: { env: { DEV: development, TAURI_ENV_DEBUG: nativeDebug } },
  });
  const cache = new Map<string, Record<string, any>>();

  function load(path: string): Record<string, any> {
    if (locals.has(path)) return locals.get(path)!;
    if (cache.has(path)) return cache.get(path)!;
    const module = { exports: {} as Record<string, any> };
    cache.set(path, module.exports);
    // Supply Vite's build environment while running real modules without native side effects.
    const compiled = ts.transpileModule(readFileSync(path, "utf8"), {
      compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.CommonJS },
      transformers: {
        before: [(transformContext) => {
          const visit: ts.Visitor = (node) =>
            ts.isMetaProperty(node) && node.keywordToken === ts.SyntaxKind.ImportKeyword
              ? ts.factory.createIdentifier("__importMeta")
              : ts.visitEachChild(node, visit, transformContext);
          return (source) => ts.visitNode(source, visit) as ts.SourceFile;
        }],
      },
    }).outputText;
    const require = (specifier: string) => {
      if (specifier.startsWith(".")) return load(resolve(dirname(path), `${specifier}.ts`));
      assert.ok(packages.has(specifier), `Unexpected package: ${specifier}`);
      return packages.get(specifier);
    };
    runInContext(`(function(require, module, exports) { ${compiled}\n})`, context, { filename: path })(
      require, module, module.exports,
    );
    return module.exports;
  }

  return {
    calls, ui, update, gate,
    session: load(resolve(sourceDir, "launch-session.ts")),
    shell: () => load(resolve(sourceDir, "render/shell.ts")),
  };
}

for (const manual of [false, true]) {
  test(`development desktop skips ${manual ? "manual" : "automatic"} update checks`, async () => {
    const { session, ui, calls } = updaterHarness(true);
    await session.checkForUpdates(manual);
    assert.equal(calls.check, 0);
    assert.equal(ui.updateState.kind, "idle");
  });
}

test("release desktop still discovers updates", async () => {
  const { session, ui, calls } = updaterHarness(false);
  await session.checkForUpdates(false);
  assert.equal(calls.check, 1);
  assert.equal(ui.updateState.kind, "available");
  assert.equal(ui.updateState.version, "99.0.0");
});

test("browser Client never calls the desktop updater", async () => {
  const { session, calls } = updaterHarness(false, false);
  await session.checkForUpdates(true);
  assert.equal(calls.check, 0);
});

for (const [label, development, desktop] of [
  ["development desktop", true, true],
  ["browser Client", false, false],
] as const) {
  test(`${label} cannot download or install a pending update`, async () => {
    const { session, ui, update, calls } = updaterHarness(development, desktop);
    ui.pendingUpdate = update;
    await session.installPendingUpdate();
    assert.equal(calls.rpc, 0);
    assert.equal(calls.download, 0);
    assert.equal(calls.install, 0);
    assert.equal(calls.relaunch, 0);
    assert.equal(ui.updateState.kind, "idle");
  });
}

test("release desktop still downloads, installs and relaunches after both gates", async () => {
  const { session, ui, update, calls } = updaterHarness(false);
  ui.pendingUpdate = update;
  await session.installPendingUpdate();
  assert.equal(calls.rpc, 2);
  assert.equal(calls.download, 1);
  assert.equal(calls.install, 1);
  assert.equal(calls.relaunch, 1);
});

test("release desktop still blocks installation when a Run is active", async () => {
  const { session, ui, update, calls, gate } = updaterHarness(false);
  ui.pendingUpdate = update;
  gate.allowed = false;
  gate.activeRunCount = 1;
  await session.installPendingUpdate();
  assert.equal(ui.updateState.kind, "blocked");
  assert.equal(calls.download, 0);
  assert.equal(calls.install, 0);
  assert.equal(calls.relaunch, 0);
});

const copy = {
  updates: "Updates",
  checkForUpdates: "Check for updates",
  updateReady: "An update is ready.",
  updateAvailable: "Update available",
  updateLater: "Later",
  updateConfirm: "Download and install",
  updateUnavailableBrowser: "Updates are unavailable in the browser.",
};

test("development desktop has no update settings", () => {
  const { shell } = updaterHarness(true);
  assert.equal(shell().updateSettings(copy), "");
});

test("development desktop cannot show a stale installation dialog", () => {
  const { shell, ui } = updaterHarness(true);
  ui.updateState = { kind: "available", version: "99.0.0" };
  assert.equal(shell().updateDialog(copy), "");
});

test("release desktop retains manual check and installation controls", () => {
  const { shell, ui } = updaterHarness(false);
  ui.updateState = { kind: "available", version: "99.0.0" };
  const renderer = shell();
  assert.match(renderer.updateSettings(copy), /data-act="check-updates"/);
  assert.match(renderer.updateDialog(copy), /data-act="install-update"/);
  assert.match(renderer.updateDialog(copy), /99\.0\.0/);
});

test("browser Client retains its unavailable notice without installation controls", () => {
  const { shell, ui } = updaterHarness(false, false);
  ui.updateState = { kind: "available", version: "99.0.0" };
  const renderer = shell();
  assert.match(renderer.updateSettings(copy), /Updates are unavailable in the browser/);
  assert.doesNotMatch(renderer.updateSettings(copy), /data-act="check-updates"/);
  assert.equal(renderer.updateDialog(copy), "");
});

for (const manual of [false, true]) {
  test(`packaged debug desktop skips ${manual ? "manual" : "automatic"} update checks`, async () => {
    const { session, ui, calls } = updaterHarness(false, true, "true");
    await session.checkForUpdates(manual);
    assert.equal(calls.check, 0);
    assert.equal(ui.updateState.kind, "idle");
  });
}

test("packaged debug desktop cannot download or install a pending update", async () => {
  const { session, ui, update, calls } = updaterHarness(false, true, "true");
  ui.pendingUpdate = update;
  await session.installPendingUpdate();
  assert.equal(calls.rpc, 0);
  assert.equal(calls.download, 0);
  assert.equal(calls.install, 0);
  assert.equal(calls.relaunch, 0);
});

test("packaged debug desktop has no update settings", () => {
  const { shell } = updaterHarness(false, true, "true");
  assert.equal(shell().updateSettings(copy), "");
});

test("packaged debug desktop cannot show a stale installation dialog", () => {
  const { shell, ui } = updaterHarness(false, true, "true");
  ui.updateState = { kind: "available", version: "99.0.0" };
  assert.equal(shell().updateDialog(copy), "");
});
