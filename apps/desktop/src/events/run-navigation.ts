import type { ReturnPoint, RpcResult, Snapshot } from "../protocol";
import { loadSelectedIssueDocument, rpc } from "../rpc";
import {
  autoFocusNewRunEnabled,
  captureReturnPoint,
  enterPrimaryPage,
  mobileClient,
} from "../view-helpers";
import { render } from "../render/app";
import { ui } from "../ui";

export async function focusRunInWorkspace(
  runId: string,
  snap: Snapshot,
  options: {
    fromProjectHistory?: boolean;
    returnPoint?: ReturnPoint | null;
  } = {},
): Promise<void> {
  const run = (snap.runs ?? []).find((candidate) => candidate.id === runId);
  const fromProjectHistory = options.fromProjectHistory ?? false;
  enterPrimaryPage("focus-workspace", snap);
  if (options.returnPoint !== undefined) ui.clientView.returnPoint = options.returnPoint;
  ui.runFocusError = null;
  ui.clientView.panels.rightSide = ui.nativeRunWindowRunId ? "hidden" : "rail";
  ui.viewingRunId = runId;
  try {
    await rpc("focusRun", { runId });
  } catch (error) {
    ui.runFocusError = {
      runId,
      message: error instanceof Error ? error.message : String(error),
    };
  }
  if (!ui.runFocusError && !fromProjectHistory && run?.issueId && ui.snapshot?.board?.selected?.id === run.issueId) {
    try {
      await loadSelectedIssueDocument();
    } catch (error) {
      const issue = ui.snapshot?.board?.selected;
      if (issue?.id === run.issueId) {
        issue.document = {
          kind: "failed",
          failure: {
            kind: "tracker",
            message: error instanceof Error ? error.message : String(error),
          },
        };
      }
    }
  }
  if (mobileClient()) {
    ui.mobileWorkspaceSection = "terminal";
    ui.mobileProjectHistoryRunOpen = fromProjectHistory;
    if (!fromProjectHistory) ui.mobileRunHistoryScope = "issue";
    ui.mobileLiveTerminal = false;
  }
  render();
}

export async function focusStartedRun(
  result: RpcResult,
  snapshotBeforeStart: Snapshot,
  options: { issueId?: string } = {},
): Promise<void> {
  const start = result.runStart;
  if (!start) return;
  if (start.status === "failed") {
    if (start.warning) {
      const issueId = options.issueId ?? snapshotBeforeStart.board?.selected?.id;
      const previousRun = issueId
        ? [...(snapshotBeforeStart.runs ?? [])].reverse().find((run) => run.issueId === issueId)
        : undefined;
      if (options.issueId) {
        ui.runStartWarning = { issueId: options.issueId, message: start.warning };
      } else if (previousRun) {
        ui.runStartWarning = { runId: previousRun.id, message: start.warning };
      }
    }
    render();
    return;
  }
  if (options.issueId && ui.runStartWarning?.issueId === options.issueId) {
    ui.runStartWarning = null;
  }
  if (!autoFocusNewRunEnabled(result.snapshot)) return;

  const returnPoint = ui.clientView.page === "focus-workspace"
    ? ui.clientView.returnPoint
    : captureReturnPoint(snapshotBeforeStart);
  await focusRunInWorkspace(start.runId, result.snapshot, { returnPoint });
}

export async function retryRunFocus(runId: string): Promise<void> {
  if (!ui.snapshot) return;
  await focusRunInWorkspace(runId, ui.snapshot, { returnPoint: ui.clientView.returnPoint });
}
