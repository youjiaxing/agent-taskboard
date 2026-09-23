import assert from "node:assert/strict";
import { chmod, mkdtemp, mkdir, readFile, symlink, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";

import {
  canonicalManifest,
  collectManifest,
  compareManifests,
  manifestDigest,
  writeManifest,
} from "./acceptance-manifest-lib.mjs";

const INPUTS = ["Cargo.lock", "src", "assets/baseline.png"];

async function fixture() {
  const root = await mkdtemp(path.join(tmpdir(), "agent-taskboard-manifest-"));
  await mkdir(path.join(root, "src"), { recursive: true });
  await mkdir(path.join(root, "assets"), { recursive: true });
  await writeFile(path.join(root, "Cargo.lock"), "lock\n");
  await writeFile(path.join(root, "src/main.ts"), "export const value = 1;\n");
  await writeFile(path.join(root, "src/tool.sh"), "#!/bin/sh\nexit 0\n");
  await chmod(path.join(root, "src/tool.sh"), 0o755);
  await symlink("main.ts", path.join(root, "src/current.ts"));
  await writeFile(path.join(root, "assets/baseline.png"), Buffer.from([0x89, 0x50, 0x4e, 0x47]));
  return root;
}

test("manifest generation is canonical and records Git semantic modes", async () => {
  const root = await fixture();
  const first = await collectManifest(root, INPUTS);
  const second = await collectManifest(root, [...INPUTS].reverse());

  assert.equal(canonicalManifest(first), canonicalManifest(second));
  assert.equal(manifestDigest(first), manifestDigest(second));
  assert.deepEqual(
    first.files.map(({ path: filePath, mode }) => [filePath, mode]),
    [
      ["Cargo.lock", "100644"],
      ["assets/baseline.png", "100644"],
      ["src/current.ts", "120000"],
      ["src/main.ts", "100644"],
      ["src/tool.sh", "100755"],
    ],
  );
});

test("exact-match comparison reports content, mode, added, and removed inputs", async () => {
  const root = await fixture();
  const accepted = await collectManifest(root, INPUTS);

  await writeFile(path.join(root, "src/main.ts"), "export const value = 2;\n");
  await chmod(path.join(root, "src/tool.sh"), 0o644);
  await writeFile(path.join(root, "src/added.ts"), "export {};\n");
  await writeFile(path.join(root, "assets/baseline.png"), Buffer.from([0x89, 0x50]));
  const current = await collectManifest(root, ["Cargo.lock", "src"]);

  assert.deepEqual(compareManifests(accepted, current), [
    "removed: assets/baseline.png",
    "added: src/added.ts",
    "changed content: src/main.ts",
    "changed mode: src/tool.sh (100755 -> 100644)",
  ]);
});

test("excluded acceptance records do not affect the manifest", async () => {
  const root = await fixture();
  const tauriRoot = path.join(root, "apps/desktop/src-tauri");
  await mkdir(path.join(tauriRoot, "src"), { recursive: true });
  await writeFile(path.join(tauriRoot, "src/lib.rs"), "pub fn run() {}\n");
  const inputs = [...INPUTS, "apps/desktop/src-tauri"];
  const before = await collectManifest(root, inputs);

  await mkdir(path.join(tauriRoot, "acceptance"), { recursive: true });
  await writeFile(path.join(tauriRoot, "acceptance/manifest.json"), "{}\n");
  await mkdir(path.join(tauriRoot, "target/release"), { recursive: true });
  await writeFile(path.join(tauriRoot, "target/release/build.log"), "generated\n");
  await writeFile(path.join(tauriRoot, "src/runtime.log"), "generated\n");
  await writeFile(path.join(tauriRoot, "src/editor.tmp"), "generated\n");

  const after = await collectManifest(root, inputs);
  assert.equal(canonicalManifest(before), canonicalManifest(after));
});

test("written checksum is the SHA-256 of the exact normalized manifest bytes", async () => {
  const root = await fixture();
  const manifest = await collectManifest(root, INPUTS);
  const outputDir = path.join(root, "acceptance");
  const digest = await writeManifest(outputDir, manifest);

  const bytes = await readFile(path.join(outputDir, "manifest.json"));
  const checksum = await readFile(path.join(outputDir, "manifest.sha256"), "utf8");
  assert.equal(bytes.toString("utf8"), canonicalManifest(manifest));
  assert.equal(checksum, `${digest}\n`);
  assert.equal(digest, manifestDigest(manifest));
});
