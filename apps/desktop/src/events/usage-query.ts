import { runFormOperation, usageCustomFormKey, usageQueryFormKey } from "../form-keys";
import type { UsageQueryRequest } from "../protocol";
import { rpc } from "../rpc";
import { ui } from "../ui";

export async function runUsageQuery(request: UsageQueryRequest, trigger?: HTMLElement): Promise<void> {
  const hostId = ui.snapshot?.focusedHostId;
  if (!hostId) return;
  const key = usageQueryFormKey(hostId);
  if (ui.formOperations.pending.has(key) || ui.formOperations.pending.has(usageCustomFormKey(hostId))) return;
  const activeId = trigger?.id ?? document.activeElement?.id ?? "";
  const focusSelector = ["usage-project", "usage-agent", "usage-model"].includes(activeId)
    ? `#${activeId}`
    : request.op === "setUsageRange"
      ? `[data-act="usage-range"][data-id="${request.extra.range}"]`
      : ui.usageQueryRetry?.focusSelector ?? "";
  ui.usageQueryRetry = { hostId, request, focusSelector };
  await runFormOperation(key, async () => {
    await rpc(request.op, request.extra);
    if (ui.usageQueryRetry?.hostId === hostId) ui.usageQueryRetry = null;
  });
  if (ui.snapshot?.focusedHostId === hostId && ui.clientView.page === "usage" && document.activeElement === document.body) {
    ui.app.querySelector<HTMLElement>(focusSelector || ".usage-ranges button.active")?.focus();
  }
}
