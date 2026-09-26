import assert from "node:assert/strict";
import test from "node:test";

import {
  compareVersions,
  nextAutomaticVersion,
} from "./auto-release-version.mjs";

test("increments the patch version after the latest release", () => {
  assert.equal(nextAutomaticVersion("0.1.2", "v0.1.2"), "0.1.3");
});

test("preserves a version that was manually advanced ahead of the latest tag", () => {
  assert.equal(nextAutomaticVersion("0.2.0", "v0.1.2"), "0.2.0");
});

test("retries an incomplete latest release instead of skipping its version", () => {
  assert.equal(nextAutomaticVersion("0.1.3", "v0.1.3", false), "0.1.3");
});

test("advances beyond a newer completed tag when source metadata is stale", () => {
  assert.equal(nextAutomaticVersion("0.1.2", "v0.1.3"), "0.1.4");
});

test("normalizes a prerelease source version before automatic publishing", () => {
  assert.equal(nextAutomaticVersion("0.2.0-beta.1", "v0.1.2"), "0.2.0");
});

test("compares semantic version components numerically", () => {
  assert.equal(compareVersions("0.1.10", "0.1.2"), 1);
  assert.equal(compareVersions("0.1.2", "0.1.10"), -1);
  assert.equal(compareVersions("0.1.2", "0.1.2"), 0);
});
