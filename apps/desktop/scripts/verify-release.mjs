import { readFile } from "node:fs/promises";
import process from "node:process";

const [config, desktopPackage, cargo] = await Promise.all([
  readJson(new URL("../src-tauri/tauri.conf.json", import.meta.url)),
  readJson(new URL("../package.json", import.meta.url)),
  readFile(new URL("../src-tauri/Cargo.toml", import.meta.url), "utf8"),
]);

const cargoVersion = cargo.match(/^version\s*=\s*"([^"]+)"/m)?.[1];
const versions = [config.version, desktopPackage.version, cargoVersion];
if (versions.some((version) => !version) || new Set(versions).size !== 1) {
  fail(`version mismatch: ${versions.join(", ")}`);
}

const tag = process.argv[2] || process.env.RELEASE_TAG;
if (tag && tag !== `v${config.version}`) {
  fail(`release tag ${tag} does not match app version v${config.version}`);
}

if (config.bundle?.active !== true) fail("bundle.active must be true");
if (JSON.stringify(config.bundle.targets) !== JSON.stringify(["dmg", "nsis"])) {
  fail("bundle.targets must contain only dmg and nsis");
}
if (config.bundle.createUpdaterArtifacts !== true) {
  fail("createUpdaterArtifacts must be true");
}
if (config.bundle.macOS?.signingIdentity !== "-") {
  fail("macOS bundles must use ad-hoc signing");
}
if (
  !config.plugins?.updater?.pubkey ||
  config.plugins.updater.pubkey.includes("PRIVATE")
) {
  fail("updater public key is missing or invalid");
}
const endpoints = config.plugins?.updater?.endpoints;
if (
  JSON.stringify(endpoints) !==
  JSON.stringify([
    "https://github.com/youjiaxing/agent-taskboard/releases/latest/download/latest.json",
  ])
) {
  fail("updater must use this repository's latest.json");
}

console.log(`release contract ok for v${config.version}`);

async function readJson(url) {
  return JSON.parse(await readFile(url, "utf8"));
}

function fail(message) {
  console.error(message);
  process.exit(1);
}
