import { deliverHostEvents } from "./main";
import { isLoopbackPage, syncLaunchDraft } from "./launch-session";
import { render } from "./render/app";
import { ui } from "./ui";
import { RpcHttpError, type ChangeScope, type IssueConflict, type RpcResult } from "./protocol";

export async function protocolBase(): Promise<string> {
  if (window.__HOST_PROTOCOL__) {
    return window.__HOST_PROTOCOL__;
  }
  if (isLoopbackPage()) {
    return "";
  }
  for (let i = 0; i < 50; i += 1) {
    if (window.__HOST_PROTOCOL__) {
      return window.__HOST_PROTOCOL__;
    }
    await new Promise((resolve) => setTimeout(resolve, 50));
  }
  throw new Error("Host protocol is not available");
}

let rpcQueue: Promise<void> = Promise.resolve();

let issueDocumentRequestSequence = 0;

export function commitRpcResult(result: RpcResult): void {
  syncLaunchDraft(result.snapshot);
  deliverHostEvents(result.events ?? [], result.snapshot);
  ui.snapshot = result.snapshot;
  if (result.viewChanges) {
    ui.changesView = result.viewChanges;
    ui.changesOpen = true;
  }
}

export async function executeRpc(
  op: string,
  extra: Record<string, unknown>,
  commit = true,
): Promise<RpcResult> {
  const response = await fetch(`${await protocolBase()}/rpc`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ op, clientInstanceId: ui.clientId, ...extra }),
  });
  if (!response.ok) {
    const text = await response.text();
    let message = text || `Host protocol ${response.status}`;
    let code = "";
    let conflict: IssueConflict | null = null;
    try {
      const parsed = JSON.parse(text) as {
        error?: string;
        message?: string;
        issueId?: string;
        fields?: string[];
        latest?: IssueConflict["latest"];
      };
      message = parsed.message || parsed.error || message;
      code = parsed.error ?? "";
      if (
        response.status === 409 &&
        code === "issue-conflict" &&
        parsed.issueId &&
        Array.isArray(parsed.fields) &&
        parsed.latest
      ) {
        conflict = {
          issueId: parsed.issueId,
          fields: parsed.fields,
          latest: parsed.latest,
        };
      }
    } catch {
      // keep raw body
    }
    throw new RpcHttpError(message, response.status, code, conflict);
  }
  const result = (await response.json()) as RpcResult;
  result.snapshot.runs = result.snapshot.runs ?? [];
  result.snapshot.focusedRunId = result.snapshot.focusedRunId ?? "";
  result.snapshot.workspaceView = result.snapshot.workspaceView ?? "project";
  if (result.snapshot.board) {
    result.snapshot.board.issueOptions = result.snapshot.board.issueOptions ?? [];
  }
  if (ui.pendingCenterView) result.snapshot.centerView = ui.pendingCenterView;
  result.snapshot.showCommandPreview = result.snapshot.showCommandPreview ?? true;
  result.snapshot.notifyDesktop = result.snapshot.notifyDesktop ?? true;
  result.snapshot.notifySound = result.snapshot.notifySound ?? true;
  result.snapshot.usageOpen = result.snapshot.usageOpen ?? false;
  result.snapshot.refreshIntervalMs = result.snapshot.refreshIntervalMs ?? 60_000;
  result.events = result.events ?? [];
  if (commit) commitRpcResult(result);
  return result;
}

export async function rpc(op: string, extra: Record<string, unknown> = {}): Promise<RpcResult> {
  const request = rpcQueue.then(() => executeRpc(op, extra));
  rpcQueue = request.then(() => undefined, () => undefined);
  return request;
}

export function rpcDetached(
  op: string,
  extra: Record<string, unknown> = {},
  commit = true,
): Promise<RpcResult> {
  return executeRpc(op, extra, commit);
}

export async function loadViewChanges(runId: string, scope: ChangeScope): Promise<void> {
  const result = await rpc("viewChanges", { runId, scope });
  ui.changesView = result.viewChanges ?? null;
}

export async function loadSelectedIssueDocument(force = false): Promise<void> {
  const issue = ui.snapshot?.board?.selected;
  if (!issue) return;
  const state = issue.document ?? { kind: "unloaded" as const };
  if (!force && state.kind !== "unloaded" && state.kind !== "loading") return;
  issue.document = state.kind === "ready" || state.kind === "stale" || state.kind === "loading"
    ? { kind: "loading", body: state.body, fetchedAtMs: state.fetchedAtMs }
    : { kind: "loading" };
  const issueId = issue.id;
  const sequence = ++issueDocumentRequestSequence;
  render();
  const result = await executeRpc("loadIssueDocument", { issueId }, false);
  if (sequence !== issueDocumentRequestSequence || ui.snapshot?.board?.selected?.id !== issueId) {
    return;
  }
  commitRpcResult(result);
}
