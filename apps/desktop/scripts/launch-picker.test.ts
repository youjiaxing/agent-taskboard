import assert from "node:assert/strict";
import { test } from "node:test";
import { syncLaunchPicker } from "../src/launch-picker.ts";

test("keeps the clicked Agent after a later snapshot still names the last successful one", () => {
  assert.deepEqual(
    syncLaunchPicker(
      { projectId: "garden", selectedAgentId: "codex" },
      { projectId: "garden", agentId: "grok-build" },
    ),
    { projectId: "garden", agentId: "grok-build" },
  );
});

test("prefills the last successful Agent when opening a picker for a new project", () => {
  assert.deepEqual(
    syncLaunchPicker(
      { projectId: "garden", selectedAgentId: "codex" },
      { projectId: "", agentId: "" },
    ),
    { projectId: "garden", agentId: "codex" },
  );
});

test("prefills only when the local picker is still empty", () => {
  assert.deepEqual(
    syncLaunchPicker(
      { projectId: "garden", selectedAgentId: "codex" },
      { projectId: "garden", agentId: "" },
    ),
    { projectId: "garden", agentId: "codex" },
  );
});
