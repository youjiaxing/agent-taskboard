import { readFile } from "node:fs/promises";
import path from "node:path";
import process from "node:process";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

import {
  canonicalManifest,
  collectManifest,
  compareManifests,
  manifestDigest,
  writeManifest,
} from "./acceptance-manifest-lib.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const desktopDir = path.resolve(scriptDir, "..");
const repositoryRoot = path.resolve(desktopDir, "../..");
const outputDir = path.join(desktopDir, "src-tauri/acceptance");
const command = process.argv[2];

try {
  if (command === "generate") {
    const { digest, manifest } = await generate();
    console.log(`acceptance manifest: ${manifest.files.length} files`);
    console.log(`acceptance manifest SHA-256: ${digest}`);
  } else if (command === "verify") {
    const digest = await verify();
    console.log(`acceptance manifest exact match: ${digest}`);
  } else if (command === "build") {
    const { digest } = await generate();
    console.log(`building acceptance candidate ${digest}`);
    const result = spawnSync(
      "npm",
      ["run", "tauri", "--", "build", "--config", "src-tauri/tauri.acceptance.conf.json"],
      {
        cwd: desktopDir,
        env: {
          ...process.env,
          AGENT_TASKBOARD_ACCEPTANCE_MANIFEST_SHA256: digest,
        },
        stdio: "inherit",
      },
    );
    if (result.error) throw result.error;
    if (result.status !== 0) process.exit(result.status ?? 1);
    await verify();
    console.log(`acceptance candidate exact match: ${digest}`);
  } else {
    throw new Error("usage: acceptance-manifest.mjs <generate|verify|build>");
  }
} catch (error) {
  console.error(error instanceof Error ? error.message : error);
  process.exit(1);
}

async function generate() {
  const manifest = await collectManifest(repositoryRoot);
  const digest = await writeManifest(outputDir, manifest);
  return { digest, manifest };
}

async function verify() {
  const manifestPath = path.join(outputDir, "manifest.json");
  const checksumPath = path.join(outputDir, "manifest.sha256");
  const [manifestText, checksumText] = await Promise.all([
    readFile(manifestPath, "utf8"),
    readFile(checksumPath, "utf8"),
  ]);
  const expected = JSON.parse(manifestText);
  const normalized = canonicalManifest(expected);
  if (normalized !== manifestText) {
    throw new Error("acceptance manifest is not in canonical form");
  }
  const digest = manifestDigest(expected);
  if (checksumText !== `${digest}\n`) {
    throw new Error(`acceptance manifest checksum mismatch: expected ${digest}`);
  }
  const current = await collectManifest(repositoryRoot);
  const differences = compareManifests(expected, current);
  if (differences.length > 0) {
    throw new Error(`acceptance manifest does not match current inputs:\n${differences.join("\n")}`);
  }
  return digest;
}
