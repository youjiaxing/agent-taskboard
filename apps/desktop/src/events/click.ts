import { eventsNeedFullRender, paintGraphEdges, renderStatusBarsOnly, reportClientView } from "../main";
import { captureGraphAnchor, effectiveClientLanguage, enterPrimaryPage, primaryPageFromSnapshot, resetGraphUiState, restoreGraphAnchor, restoreReturnPointMemory, syncReturnPointNavigation } from "../view-helpers";
import type { AppearancePreference, CenterView, FormKey, Language, MobileWorkspaceSection, RpcResult, SetAppearancePreferenceRequest, Snapshot } from "../protocol";
import { checkForUpdates, chooseProjectDirectory, desktopShellAvailable, expectedOpening, inferFromLocalPath, installPendingUpdate, loadStartupSettings, openExternalUrl, openRunWindow, setHostMode, supersedeProjectInference, syncLaunchDraft } from "../launch-session";
import { issueDraftKey, clearFormOperation, editableIssueBody, editableIssueRelations, issueBlockersFormKey, issueCreateFormKey, issueEditFormKey, issueOpenFormKey, revokeClientFormKey, runFormOperation, usageCustomFormKey } from "../form-keys";
import { APPEARANCE_PREFERENCES, browserClient, ensureBrowserAppearance, mobileClient, saveBrowserAppearance, workspaceRun } from "../view-helpers";
import { saveClientPanelState } from "../workbench";
import { loadSelectedIssueDocument, loadViewChanges, rpc, rpcDetached } from "../rpc";
import { parsePairingPayload, safeHttpUrl } from "../client-utils";
import { render } from "../render/app";
import { emptyDraft, ui } from "../ui";
import { rememberDialogTrigger, restoreDialogTrigger } from "../components/dialog-controller";

function leaveSettingsPage(): void {
  if (!ui.snapshot || ui.clientView.page !== "settings") return;
  ui.clientView.page = primaryPageFromSnapshot(ui.snapshot);
  ui.clientView.returnPoint = null;
  ui.returnPointHistory.length = 0;
}

/** The mobile drawer closes as soon as it hands over to a page, a dialog, or a confirmation. */
function closeMobileDrawer(): void {
  ui.mobileDrawerOpen = false;
  ui.mobileDrawerAppearanceOpen = false;
}

/** The mobile bottom navigation switches primary views; it is not a return-point stack. */
async function openMobilePage(page: "board" | "focus-workspace"): Promise<void> {
  if (!ui.snapshot) return;
  ui.clientView.returnPoint = null;
  ui.returnPointHistory.length = 0;
  ui.clientView.page = page;
  ui.mobileLiveTerminal = false;
  if (page === "board") {
    // The Host mirror also owns the workspace view; leaving it set would pull the client back in.
    if (ui.snapshot.workspaceView !== "project") await rpc("returnToBoard");
  } else if (!ui.snapshot.board?.selected) {
    const run = workspaceRun(ui.snapshot);
    if (run) await rpc("focusRun", { runId: run.id });
  }
  render();
}

async function dismissDialog(dialogId: string): Promise<void> {
  if (dialogId === "mobile-drawer") {
    closeMobileDrawer();
  } else if (dialogId === "mobile-search") {
    ui.mobileSearchOpen = false;
  } else if (dialogId === "pairing") {
    if (ui.formOperations.pending.has("pairing")) return;
    ui.pairingOpen = false;
    ui.pairingError = "";
  } else if (dialogId === "project-form") {
    if (ui.projectOperation) return;
    ui.formOpen = null;
    supersedeProjectInference();
    ui.formError = "";
  } else if (dialogId === "launch") {
    if (ui.snapshot?.launchForm && ui.formOperations.pending.has(`launch:${ui.snapshot.launchForm.projectId}`)) return;
    await rpc("cancelRunLaunch");
    ui.launchDraft = null;
    ui.launchPickerProjectId = "";
    ui.launchPickerAgentId = "";
    ui.launchPreviewSequence += 1;
    if (ui.launchPreviewTimer != null) window.clearTimeout(ui.launchPreviewTimer);
  } else if (dialogId === "remove-project") {
    if (ui.projectOperation) return;
    ui.removeProject = null;
    ui.removeError = "";
  } else if (dialogId === "quit-host") {
    await rpc("cancelQuit");
  } else if (dialogId === "update") {
    if (ui.updateState.kind === "installing") return;
    ui.updateState = { kind: "idle" };
  } else if (dialogId === "keyboard-help") {
    ui.keyboardHelpOpen = false;
  } else if (dialogId === "stop-run" || dialogId === "revoke-client") {
    if (ui.confirmationPending) return;
    ui.dangerConfirmation = null;
    ui.confirmationError = "";
    ui.confirmationPending = false;
  } else {
    return;
  }
  render();
  restoreDialogTrigger(dialogId);
}

export async function openSettingsPanel(): Promise<void> {
  if (!ui.snapshot) return;
  enterPrimaryPage("settings", ui.snapshot);
  await loadStartupSettings();
  ui.pairingOpen = false;
  ui.formOpen = null;
  ui.removeProject = null;
  ui.projectMenuId = "";
  render();
}

export async function returnToPreviousPage(): Promise<void> {
  if (!ui.snapshot) return;
  const returnPoint = ui.clientView.returnPoint;
  if (!returnPoint) {
    ui.clientView.page = primaryPageFromSnapshot(ui.snapshot);
    ui.returnPointHistory.length = 0;
    render();
    return;
  }
  if (returnPoint.hostId && returnPoint.hostId !== ui.snapshot.focusedHostId) {
    await rpc("focusHost", { hostId: returnPoint.hostId });
  }
  if (returnPoint.projectId && returnPoint.projectId !== ui.snapshot.focusedProjectId) {
    await rpc("focusProject", { projectId: returnPoint.projectId });
  }
  if (returnPoint.page === "host-overview") {
    await rpc("openHostOverview");
  } else if (returnPoint.page === "usage") {
    await rpc("openUsage");
  } else if (returnPoint.page === "focus-workspace" && returnPoint.runId) {
    await rpc("focusRun", { runId: returnPoint.runId });
  } else {
    if (ui.snapshot.usageOpen) await rpc("closeUsage");
    if (ui.snapshot.workspaceView !== "project") await rpc("returnToBoard");
    const view = returnPoint.page === "dependency-graph" ? "graph" : "board";
    if (ui.snapshot.centerView !== view) await rpc("setCenterView", { view });
    if (returnPoint.issueId && ui.snapshot.board?.selected?.id !== returnPoint.issueId) {
      await rpc("focusIssue", { issueId: returnPoint.issueId });
      await loadSelectedIssueDocument();
    }
  }
  if (ui.clientView.returnPoint !== returnPoint) return;
  restoreReturnPointMemory(returnPoint);
  render();
}

export function openKeyboardHelp(): void {
  ui.keyboardHelpOpen = true;
  render();
}

let deferredLaunchDiscoverySequence = 0;

async function completeDeferredLaunchDiscovery(
  projectId: string,
  issueId: string | undefined,
  agentId: string,
): Promise<void> {
  const sequence = ++deferredLaunchDiscoverySequence;
  try {
    const result = await rpcDetached("prepareRunLaunch", {
      projectId,
      issueId,
      agentId,
      language: effectiveClientLanguage(),
    }, false);
    if (
      sequence !== deferredLaunchDiscoverySequence
      || ui.snapshot?.launchForm?.projectId !== projectId
      || ui.snapshot.launchForm.issueId !== issueId
      || ui.snapshot.launchForm.selectedAgentId !== agentId
    ) {
      return;
    }
    const draft = ui.launchDraft;
    const previousValues = draft?.values ?? {};
    const previousOpening = draft?.openingText;
    ui.snapshot = { ...ui.snapshot, launchForm: result.snapshot.launchForm };
    syncLaunchDraft(ui.snapshot);
    if (ui.launchDraft) {
      ui.launchDraft.values = { ...ui.launchDraft.values, ...previousValues };
      ui.launchDraft.openingText = previousOpening ?? ui.launchDraft.openingText;
    }
    render();
  } catch {
    if (
      sequence === deferredLaunchDiscoverySequence
      && ui.snapshot?.launchForm?.projectId === projectId
      && ui.snapshot.launchForm.issueId === issueId
      && ui.snapshot.launchForm.selectedAgentId === agentId
      && ui.snapshot.launchForm.optionDiscoveryPending
    ) {
      ui.snapshot = {
        ...ui.snapshot,
        launchForm: { ...ui.snapshot.launchForm, optionDiscoveryPending: null },
      };
      render();
    }
  }
}

export async function handleAppClick(event: MouseEvent): Promise<void> {
  if (!ui.snapshot) return;
  const target = (event.target as HTMLElement).closest<HTMLElement>("[data-act]");
  if (!target) {
    if (ui.appearanceMenuOpen || ui.moreMenuOpen || ui.projectMenuId) {
      ui.appearanceMenuOpen = false;
      ui.moreMenuOpen = false;
      ui.projectMenuId = "";
      render();
    }
    return;
  }
  if (target.dataset.stop) event.stopPropagation();
  const act = target.dataset.act;
  if (act === "dismiss-dialog") {
    if (target.classList.contains("dialog-backdrop") || target.classList.contains("drawer-backdrop")) {
      if (event.target !== target) return;
    }
    await dismissDialog(target.dataset.dialogId ?? target.closest<HTMLElement>("[data-dialog-id]")?.dataset.dialogId ?? "");
    return;
  }
  if (act !== "appearance-menu" && act !== "appearance") ui.appearanceMenuOpen = false;
  if (act !== "more-menu") ui.moreMenuOpen = false;
  if (act !== "project-menu") ui.projectMenuId = "";
  if (act === "mobile-drawer") {
    rememberDialogTrigger("mobile-drawer", target);
    ui.mobileDrawerOpen = true;
    render();
    return;
  }
  if (act === "mobile-nav" && target.dataset.id) {
    await openMobilePage(target.dataset.id as "board" | "focus-workspace");
    return;
  }
  if (act === "mobile-workspace-section" && target.dataset.id) {
    ui.mobileWorkspaceSection = target.dataset.id as MobileWorkspaceSection;
    ui.mobileLiveTerminal = false;
    render();
    return;
  }
  if (act === "mobile-issue-entry" || act === "mobile-history-entry") {
    rememberDialogTrigger(
      "mobile-drawer",
      target,
      "button[data-act='mobile-drawer']",
    );
    closeMobileDrawer();
    enterPrimaryPage("focus-workspace", ui.snapshot);
    ui.mobileWorkspaceSection = act === "mobile-issue-entry" ? "issue" : "runs";
    ui.mobileLiveTerminal = false;
    render();
    return;
  }
  if (act === "mobile-search-entry") {
    rememberDialogTrigger("mobile-search", target, "button[data-act='mobile-drawer']");
    closeMobileDrawer();
    ui.mobileSearchOpen = true;
    render();
    return;
  }
  if (act === "mobile-appearance-entry") {
    ui.mobileDrawerAppearanceOpen = !ui.mobileDrawerAppearanceOpen;
    render();
    return;
  }
  if (act === "mobile-settings-entry") {
    closeMobileDrawer();
    await openSettingsPanel();
    return;
  }
  if (act === "mobile-live-terminal") {
    ui.mobileLiveTerminal = true;
    render();
    return;
  }
  if (act === "return-page") {
    await returnToPreviousPage();
    return;
  }
  if (act === "toggle-sidebar") {
    ui.clientView.panels.sidebarVisible = !ui.clientView.panels.sidebarVisible;
    saveClientPanelState();
    render();
    return;
  }
  if (act === "toggle-issue") {
    ui.clientView.panels.rightSide = ui.clientView.panels.rightSide === "rail" ? "hidden" : "rail";
    saveClientPanelState();
    render();
    return;
  }
  if (act === "open-overview") {
    closeMobileDrawer();
    enterPrimaryPage("host-overview", ui.snapshot);
    await rpc("openHostOverview");
    render();
    return;
  }
  if (act === "settings") {
    await openSettingsPanel();
    return;
  }
  if (act === "host-mode" && target.dataset.id) {
    const mode = target.dataset.id as Snapshot["hostMode"];
    if (mode !== ui.snapshot.hostMode) {
      await setHostMode(mode);
    }
    render();
    return;
  }
  if (act === "refresh-launch-environment") {
    ui.launchEnvironmentError = "";
    try {
      const result = await rpc("refreshLaunchEnvironment");
      ui.launchEnvironmentState = result.launchEnvironment ?? ui.launchEnvironmentState;
    } catch (error) {
      ui.launchEnvironmentState = {
        status: "failed",
        refreshedDirectories: 0,
      };
      ui.launchEnvironmentError = error instanceof Error ? error.message : String(error);
    }
    render();
    return;
  }
  if (act === "check-updates") {
    rememberDialogTrigger("update", target);
    await checkForUpdates(true);
    return;
  }
  if (act === "install-update") {
    await installPendingUpdate();
    return;
  }
  if (act === "pair") {
    rememberDialogTrigger("pairing", target, target.closest("[data-dialog-id='mobile-drawer']") ? "button[data-act='mobile-drawer']" : undefined);
    closeMobileDrawer();
    ui.pairingOpen = true;
    leaveSettingsPage();
    ui.hostPickerOpen = false;
    ui.formOpen = null;
    ui.removeProject = null;
    ui.projectMenuId = "";
    ui.pairingError = "";
    render();
    return;
  }
  if (act === "register") {
    rememberDialogTrigger("project-form", target, target.closest("[data-dialog-id='mobile-drawer']") ? "button[data-act='mobile-drawer']" : undefined);
    closeMobileDrawer();
    ui.formOpen = "register";
    ui.formProjectId = "";
    ui.formDraft = emptyDraft();
    ui.autoFilledProjectName = "";
    supersedeProjectInference();
    ui.formError = "";
    ui.removeError = "";
    ui.projectMenuId = "";
    ui.pairingOpen = false;
    leaveSettingsPage();
    ui.removeProject = null;
    render();
    return;
  }
  if (act === "new-issue" && target.dataset.id) {
    ui.createIssueProjectId = target.dataset.id;
    ui.createIssueDraft = { title: "", body: "" };
    ui.createIssueOpen = true;
    clearFormOperation(issueCreateFormKey(ui.createIssueProjectId));
    ui.issueEditOpenIds.delete(issueDraftKey(ui.snapshot.board?.selected?.id ?? ""));
    render();
    ui.app.querySelector<HTMLInputElement>("#issue-create-title")?.focus();
    return;
  }
  if (act === "cancel-new-issue") {
    if (ui.createIssueProjectId && ui.formOperations.pending.has(issueCreateFormKey(ui.createIssueProjectId))) return;
    ui.createIssueOpen = false;
    ui.createIssueProjectId = "";
    ui.createIssueDraft = { title: "", body: "" };
    render();
    return;
  }
  if (act === "keyboard-help") {
    rememberDialogTrigger("keyboard-help", target, "button[data-act='more-menu']");
    openKeyboardHelp();
    return;
  }
  if (act === "open-issue" && target.dataset.url) {
    await openExternalUrl(target.dataset.url);
    return;
  }
  if (act === "open-external" && target.dataset.url) {
    event.preventDefault();
    const url = safeHttpUrl(target.dataset.url);
    if (url) await openExternalUrl(url);
    return;
  }
  if (act === "retry-issue-document") {
    await loadSelectedIssueDocument(true);
    render();
    return;
  }
  if (act === "use-latest-issue-conflict" && target.dataset.formKey) {
    const key = target.dataset.formKey as FormKey;
    const conflict = ui.formOperations.conflicts.get(key);
    if (!conflict) return;
    if (key.startsWith("issue-edit:")) {
      const draft = ui.issueEditDrafts.get(issueDraftKey(conflict.issueId));
      if (draft) {
        ui.issueEditDrafts.set(issueDraftKey(conflict.issueId), {
          ...draft,
          title: conflict.fields.includes("title") ? conflict.latest.title ?? draft.title : draft.title,
          body: conflict.fields.includes("body") ? conflict.latest.body ?? draft.body : draft.body,
          baseTitle: conflict.fields.includes("title")
            ? conflict.latest.title ?? draft.baseTitle
            : draft.baseTitle,
          baseBody: conflict.fields.includes("body")
            ? conflict.latest.body ?? draft.baseBody
            : draft.baseBody,
        });
      }
    } else if (key.startsWith("issue-parent:")) {
      const draft = ui.issueRelationDrafts.get(issueDraftKey(conflict.issueId));
      if (draft) {
        const parent = conflict.latest.parent ?? "";
        ui.issueRelationDrafts.set(issueDraftKey(conflict.issueId), {
          ...draft,
          parent,
          baseParent: parent,
        });
      }
    } else if (key.startsWith("issue-blockers:")) {
      const draft = ui.issueRelationDrafts.get(issueDraftKey(conflict.issueId));
      if (draft) {
        const blockedBy = conflict.latest.blockedBy ?? [];
        ui.issueRelationDrafts.set(issueDraftKey(conflict.issueId), {
          ...draft,
          blockedBy: [...blockedBy],
          baseBlockedBy: [...blockedBy],
        });
      }
    }
    clearFormOperation(key);
    render();
    return;
  }
  if (act === "edit-issue" && target.dataset.id) {
    const issue = ui.snapshot.board?.selected?.id === target.dataset.id
      ? ui.snapshot.board.selected
      : null;
    if (!issue) return;
    if (issue.document.kind === "unloaded") {
      await loadSelectedIssueDocument();
    }
    const current = ui.snapshot.board?.selected?.id === issue.id ? ui.snapshot.board.selected : issue;
    if (current.document.kind !== "ready") return;
    const title = current.title;
    const body = editableIssueBody(current);
    ui.issueEditDrafts.set(issueDraftKey(issue.id), {
      title,
      body,
      baseTitle: title,
      baseBody: body,
    });
    ui.issueEditOpenIds.add(issueDraftKey(issue.id));
    const workspaceSections = ui.workspaceRailOpenSections.get(issue.id);
    workspaceSections?.add("issue");
    clearFormOperation(issueEditFormKey(issue.id));
    render();
    ui.app.querySelector<HTMLInputElement>("#issue-edit-title")?.focus();
    return;
  }
  if (act === "cancel-edit-issue" && target.dataset.id) {
    if (ui.formOperations.pending.has(issueEditFormKey(target.dataset.id))) return;
    ui.issueEditOpenIds.delete(issueDraftKey(target.dataset.id ?? ui.snapshot.board?.selected?.id ?? ""));
    render();
    return;
  }
  if (act === "clear-issue-blockers" && target.dataset.id) {
    const issue = ui.snapshot.board?.selected?.id === target.dataset.id ? ui.snapshot.board.selected : null;
    if (!issue || ui.formOperations.pending.has(issueBlockersFormKey(issue.id))) return;
    const current = editableIssueRelations(issue);
    ui.issueRelationDrafts.set(issueDraftKey(issue.id), { ...current, blockedBy: [] });
    render();
    return;
  }
  if (act === "toggle-issue-open" && target.dataset.id) {
    const issue = ui.snapshot.board?.selected?.id === target.dataset.id ? ui.snapshot.board.selected : null;
    if (!issue) return;
    const key = issueOpenFormKey(issue.id);
    await runFormOperation(key, async () => {
      await rpcDetached("setIssueOpen", { issueId: issue.id, open: !issue.open });
    });
    return;
  }
  if (act === "project-menu" && target.dataset.id) {
    ui.projectMenuId = ui.projectMenuId === target.dataset.id ? "" : target.dataset.id;
    render();
    return;
  }
  if (act === "focus-project" && target.dataset.id) {
    ui.projectMenuId = "";
    closeMobileDrawer();
    ui.clientView.page = "board";
    ui.clientView.returnPoint = null;
    ui.returnPointHistory.length = 0;
    await rpc("focusProject", { projectId: target.dataset.id });
    render();
    return;
  }
  if (act === "new-run" && target.dataset.id) {
    rememberDialogTrigger("launch", target);
    const projectId = target.dataset.id;
    ui.projectMenuId = "";
    leaveSettingsPage();
    ui.pairingOpen = false;
    ui.formOpen = null;
    ui.launchDraft = null;
    await rpc("prepareRunLaunch", {
      projectId,
      deferDiscovery: true,
      language: effectiveClientLanguage(),
    });
    render();
    const selectedAgentId = ui.snapshot.launchForm?.selectedAgentId;
    if (selectedAgentId) {
      void completeDeferredLaunchDiscovery(projectId, undefined, selectedAgentId);
    }
    return;
  }
  if (act === "execute-run" && target.dataset.id && ui.snapshot.focusedProjectId) {
    rememberDialogTrigger("launch", target);
    leaveSettingsPage();
    ui.pairingOpen = false;
    ui.formOpen = null;
    ui.launchDraft = null;
    await rpc("focusIssue", { issueId: target.dataset.id });
    await rpc("prepareRunLaunch", {
      projectId: ui.snapshot.focusedProjectId,
      issueId: target.dataset.id,
      deferDiscovery: true,
      language: effectiveClientLanguage(),
    });
    render();
    const selectedAgentId = ui.snapshot.launchForm?.selectedAgentId;
    if (selectedAgentId) {
      void completeDeferredLaunchDiscovery(
        ui.snapshot.focusedProjectId,
        target.dataset.id,
        selectedAgentId,
      );
    }
    return;
  }
  if (act === "continue-run" && target.dataset.id) {
    await rpc("continueRun", { issueId: target.dataset.id });
    render();
    return;
  }
  if (act === "release-claim" && target.dataset.id) {
    await rpcDetached("releaseIssue", { issueId: target.dataset.id });
    render();
    return;
  }
  if (act === "switch-agent") {
    const form = ui.snapshot.launchForm;
    if (!form) return;
    ui.launchDraft = null;
    await rpc("prepareRunLaunch", {
      projectId: form.projectId,
      issueId: form.issueId,
      pickAgent: true,
      language: effectiveClientLanguage(),
    });
    render();
    return;
  }
  if (act === "select-agent" && target.dataset.id) {
    ui.launchPickerAgentId = target.dataset.id;
    render();
    return;
  }
  if (act === "next-agent" && ui.launchPickerAgentId) {
    const form = ui.snapshot.launchForm;
    if (!form) return;
    const agentId = ui.launchPickerAgentId;
    const issueId = form.issueId ?? undefined;
    ui.launchDraft = null;
    await rpc("prepareRunLaunch", {
      projectId: form.projectId,
      issueId,
      agentId,
      deferDiscovery: true,
      language: effectiveClientLanguage(),
    });
    render();
    void completeDeferredLaunchDiscovery(form.projectId, issueId, agentId);
    return;
  }
  if (act === "intent") {
    if (!ui.launchDraft || !ui.snapshot.launchForm) return;
    const intentId = target.dataset.id ?? "";
    ui.launchDraft.intentId = intentId;
    ui.launchDraft.custom = false;
    ui.launchDraft.openingText = expectedOpening(ui.snapshot.launchForm, ui.launchDraft);
    render();
    return;
  }
  if (act === "intent-custom") {
    return;
  }
  if (act === "toggle-folded") {
    ui.launchFolded = !ui.launchFolded;
    return;
  }
  if (act === "focus-run" && target.dataset.id) {
    enterPrimaryPage("focus-workspace", ui.snapshot);
    ui.clientView.panels.rightSide = ui.nativeRunWindowRunId ? "hidden" : "rail";
    await rpc("focusRun", { runId: target.dataset.id });
    if (ui.clientView.page === "focus-workspace") syncReturnPointNavigation();
    await loadSelectedIssueDocument();
    if (mobileClient()) {
      ui.mobileWorkspaceSection = "terminal";
      ui.mobileLiveTerminal = false;
    }
    render();
    return;
  }
  if (act === "open-run-window" && target.dataset.id) {
    const run = (ui.snapshot.runs ?? []).find((item) => item.id === target.dataset.id);
    if (!run || run.status === "ended") return;
    try {
      await openRunWindow(
        run.id,
        ui.snapshot.focusedHostId,
        run.projectId,
        `${run.agentName} · ${run.issueId ?? ui.snapshot.copy.unboundIssue}`,
      );
    } catch (error) {
      console.warn("unable to open Run window", error);
    }
    return;
  }
  if (act === "open-usage") {
    closeMobileDrawer();
    leaveSettingsPage();
    ui.pairingOpen = false;
    ui.formOpen = null;
    enterPrimaryPage("usage", ui.snapshot);
    await rpc("openUsage");
    render();
    return;
  }
  if (act === "usage-range" && target.dataset.id) {
    ui.usageCustomDraft = null;
    clearFormOperation(usageCustomFormKey(ui.snapshot.focusedHostId));
    await rpc("setUsageRange", { range: target.dataset.id });
    render();
    return;
  }
  if (act === "open-usage-run" && target.dataset.id) {
    enterPrimaryPage("usage", ui.snapshot);
    await rpc("openUsageForRun", { runId: target.dataset.id });
    render();
    return;
  }
  if (act === "open-run-usage" && target.dataset.id) {
    enterPrimaryPage("focus-workspace", ui.snapshot);
    ui.clientView.panels.rightSide = "rail";
    await rpc("openRunFromUsage", { runId: target.dataset.id });
    render();
    return;
  }
  if (act === "toggle-telemetry") {
    ui.telemetryExpanded = !ui.telemetryExpanded;
    render();
    return;
  }
  if (act === "stop-run" && target.dataset.id) {
    rememberDialogTrigger("stop-run", target);
    ui.dangerConfirmation = {
      kind: "stop-run",
      runId: target.dataset.id,
    };
    ui.confirmationError = "";
    ui.confirmationPending = false;
    render();
    return;
  }
  if (act === "confirm-stop-run" && ui.dangerConfirmation?.kind === "stop-run") {
    if (ui.confirmationPending) return;
    const confirmation = ui.dangerConfirmation;
    ui.confirmationPending = true;
    render();
    try {
      await rpc("stopRun", { runId: confirmation.runId });
      ui.dangerConfirmation = null;
      ui.confirmationError = "";
      ui.confirmationPending = false;
      render();
      restoreDialogTrigger("stop-run");
    } catch (error) {
      ui.confirmationPending = false;
      ui.confirmationError = error instanceof Error ? error.message : String(error);
      render();
    }
    return;
  }
  if (act === "view-changes" && target.dataset.id) {
    if (ui.clientView.panels.rightSide === "changes") {
      ui.clientView.panels.rightSide = "rail";
      saveClientPanelState();
      render();
      return;
    }
    ui.clientView.panels.rightSide = "changes";
    saveClientPanelState();
    ui.changesScope = "this-round";
    if (ui.changesView?.runId !== target.dataset.id) ui.changesView = null;
    render();
    await loadViewChanges(target.dataset.id, ui.changesScope);
    render();
    return;
  }
  if (act === "changes-scope" && target.dataset.id) {
    const scope = target.dataset.id === "uncommitted" ? "uncommitted" : "this-round";
    ui.changesScope = scope;
    const runId = ui.changesView?.runId ?? workspaceRun(ui.snapshot)?.id;
    if (runId) await loadViewChanges(runId, scope);
    render();
    return;
  }
  if (act === "note-line" && target.dataset.repo && target.dataset.path && target.dataset.line) {
    ui.noteTarget = {
      repo: target.dataset.repo,
      path: target.dataset.path,
      line: Number(target.dataset.line),
    };
    ui.noteDraft = "";
    render();
    const input = ui.app.querySelector<HTMLInputElement>(".note-form input");
    input?.focus();
    return;
  }
  if (act === "delete-note" && target.dataset.id) {
    await rpc("deleteChangeNote", { noteId: target.dataset.id });
    const runId = ui.changesView?.runId ?? workspaceRun(ui.snapshot)?.id;
    if (runId) await loadViewChanges(runId, ui.changesScope);
    render();
    return;
  }
  if (act === "confirm-quit") {
    await rpc("confirmQuitStopAll");
    render();
    return;
  }
  if (act === "refresh") {
    ui.refreshing = true;
    renderStatusBarsOnly();
    let result: RpcResult | null = null;
    try {
      result = await rpcDetached("refresh", { projectId: ui.snapshot.focusedProjectId });
    } finally {
      ui.refreshing = false;
    }
    if (eventsNeedFullRender(result?.events ?? [])) render();
    else renderStatusBarsOnly();
    return;
  }
  if (act === "edit-project" && target.dataset.id) {
    rememberDialogTrigger(
      "project-form",
      target,
      target.closest("[data-dialog-id='mobile-drawer']")
        ? "button[data-act='mobile-drawer']"
        : `button[data-act='project-menu'][data-id='${CSS.escape(target.dataset.id)}']`,
    );
    closeMobileDrawer();
    const project = ui.snapshot.projects.find((item) => item.id === target.dataset.id);
    if (!project) return;
    ui.formOpen = "edit";
    ui.formProjectId = project.id;
    ui.formDraft = {
      name: project.name,
      localPath: project.localPath,
      githubHost: project.githubHost,
      repository: project.repository,
    };
    ui.autoFilledProjectName = "";
    supersedeProjectInference();
    ui.formError = "";
    ui.removeError = "";
    ui.projectMenuId = "";
    render();
    return;
  }
  if (act === "remove-project" && target.dataset.id) {
    rememberDialogTrigger(
      "remove-project",
      target,
      target.closest("[data-dialog-id='mobile-drawer']")
        ? "button[data-act='mobile-drawer']"
        : `button[data-act='project-menu'][data-id='${CSS.escape(target.dataset.id)}']`,
    );
    closeMobileDrawer();
    ui.removeProject = ui.snapshot.projects.find((item) => item.id === target.dataset.id) ?? null;
    ui.removeError = "";
    ui.projectMenuId = "";
    render();
    return;
  }
  if (act === "confirm-remove" && ui.removeProject) {
    if (ui.projectOperation) return;
    ui.removeError = "";
    ui.projectOperation = "remove";
    render();
    try {
      await rpc("removeProject", { projectId: ui.removeProject.id });
      ui.removeProject = null;
    } catch (error) {
      ui.removeError = error instanceof Error ? error.message : String(error);
    } finally {
      ui.projectOperation = null;
    }
    render();
    return;
  }
  if (act === "choose-project-directory") {
    event.preventDefault();
    event.stopPropagation();
    await chooseProjectDirectory();
    return;
  }
  if (act === "apply-infer" && ui.projectInference.status === "candidate") {
    const candidate = ui.projectInference.candidate;
    const useCandidateName = !ui.formDraft.name.trim() || ui.formDraft.name.trim() === ui.autoFilledProjectName;
    ui.formDraft = {
      name: useCandidateName ? candidate.name : ui.formDraft.name,
      localPath: candidate.localPath,
      githubHost: candidate.githubHost,
      repository: candidate.repository,
    };
    ui.autoFilledProjectName = useCandidateName ? candidate.name : "";
    ui.projectInference = { status: "idle", requestId: ui.projectInference.requestId };
    render();
    return;
  }
  if (act === "retry-infer") {
    await inferFromLocalPath(ui.formDraft.localPath);
    return;
  }
  if (act === "toggle-hosts") {
    ui.hostPickerOpen = ui.snapshot.hosts.length > 1 ? !ui.hostPickerOpen : false;
    render();
    return;
  }
  if (act === "focus-host" && target.dataset.id) {
    await reportClientView(false);
    await rpc("focusHost", { hostId: target.dataset.id });
    ui.hostPickerOpen = false;
    await reportClientView();
    render();
    return;
  }
  if (act === "show-offer") {
    if (ui.formOperations.pending.has("pairing")) return;
    ui.pairingError = "";
    const addressInput = ui.app.querySelector<HTMLInputElement>("[data-field='address']");
    ui.pairingAddress = addressInput?.value ?? ui.pairingAddress;
    await runFormOperation("pairing", async () => {
      await rpc("beginPairingOffer", { address: ui.pairingAddress });
    });
    return;
  }
  if (act === "copy-offer" && ui.snapshot.pairingOffer) {
    await navigator.clipboard.writeText(ui.snapshot.pairingOffer.text);
    return;
  }
  if (act === "revoke" && target.dataset.id) {
    rememberDialogTrigger("revoke-client", target);
    ui.dangerConfirmation = {
      kind: "revoke-client",
      clientId: target.dataset.id,
      clientName: target.dataset.name ?? target.closest(".client-row")?.querySelector("span")?.textContent?.trim() ?? target.dataset.id,
    };
    ui.confirmationError = "";
    ui.confirmationPending = false;
    render();
    return;
  }
  if (act === "confirm-revoke-client" && ui.dangerConfirmation?.kind === "revoke-client") {
    if (ui.confirmationPending) return;
    const confirmation = ui.dangerConfirmation;
    ui.confirmationPending = true;
    const success = await runFormOperation(revokeClientFormKey(confirmation.clientId), async () => {
      await rpc("revokeClient", { clientId: confirmation.clientId });
    });
    ui.confirmationPending = false;
    if (success) {
      ui.dangerConfirmation = null;
      ui.confirmationError = "";
      render();
      restoreDialogTrigger("revoke-client");
    } else {
      ui.confirmationError = ui.formOperations.errors.get(revokeClientFormKey(confirmation.clientId)) ?? "";
      render();
    }
    return;
  }
  if (act === "connect-host") {
    if (ui.formOperations.pending.has("pairing")) return;
    ui.pairingError = "";
    const pasteInput = ui.app.querySelector<HTMLTextAreaElement>("[data-field='paste']");
    ui.pairingPaste = pasteInput?.value ?? ui.pairingPaste;
    const parsed = parsePairingPayload(ui.pairingPaste);
    if (!parsed) {
      ui.pairingError = ui.snapshot.copy.pairingPaste;
      render();
      return;
    }
    await runFormOperation("pairing", async () => {
      await rpc("pairRemoteHost", parsed);
      ui.pairingPaste = "";
      ui.pairingOpen = false;
    });
    return;
  }
  if (act === "language" && target.dataset.id) {
    if (browserClient()) {
      const appearance = ensureBrowserAppearance();
      saveBrowserAppearance({ ...appearance, language: target.dataset.id as Language });
    } else {
      await rpc("setLanguage", { language: target.dataset.id });
    }
    render();
    return;
  }
  if (act === "more-menu") {
    ui.moreMenuOpen = !ui.moreMenuOpen;
    render();
    if (ui.moreMenuOpen) ui.app.querySelector<HTMLButtonElement>(".more-menu button")?.focus();
    return;
  }
  if (act === "appearance-menu") {
    ui.appearanceMenuOpen = !ui.appearanceMenuOpen;
    render();
    if (ui.appearanceMenuOpen) {
      ui.app.querySelector<HTMLButtonElement>(".appearance-menu button")?.focus();
    }
    return;
  }
  if (act === "appearance" && target.dataset.id) {
    const appearancePreference = target.dataset.id as AppearancePreference;
    if (!APPEARANCE_PREFERENCES.includes(appearancePreference)) return;
    if (desktopShellAvailable()) {
      const request: SetAppearancePreferenceRequest = { appearancePreference };
      await rpc("setAppearancePreference", request);
    } else {
      saveBrowserAppearance({ ...ensureBrowserAppearance(), appearancePreference });
    }
    ui.appearanceMenuOpen = false;
    render();
    ui.app.querySelector<HTMLButtonElement>("button[data-act='appearance-menu']")?.focus();
    return;
  }
  if (act === "quit") {
    rememberDialogTrigger("quit-host", target);
    await rpc("quitHost");
    render();
    return;
  }
  if (act === "veto-advance" && target.dataset.id) {
    await rpc("vetoPendingConfirmation", { projectId: target.dataset.id });
    render();
    return;
  }
  if (act === "center-view" && target.dataset.id) {
    const view = target.dataset.id as CenterView;
    ui.clientView.page = view === "graph" ? "dependency-graph" : "board";
    ui.pendingCenterView = view;
    ui.snapshot.centerView = view;
    if (view === "graph") {
      resetGraphUiState();
    }
    render();
    try {
      await rpc("setCenterView", { view });
    } finally {
      ui.pendingCenterView = null;
    }
    render();
    return;
  }
  if (act === "center-graph" && target.dataset.id) {
    const graphAnchor = captureGraphAnchor(target.dataset.id);
    ui.pendingGraphAnchor = graphAnchor;
    await rpc("centerDependencyGraph", { issueId: target.dataset.id });
    render();
    await loadSelectedIssueDocument();
    render();
    if (graphAnchor) {
      const canvas = ui.app?.querySelector<HTMLElement>(".graph-canvas");
      if (canvas && restoreGraphAnchor(canvas, graphAnchor)) paintGraphEdges();
    }
    return;
  }
  if (act === "graph-overview") {
    ui.pendingGraphAnchor = null;
    resetGraphUiState();
    await rpc("showDependencyGraphOverview");
    render();
    return;
  }
  if (act === "view-dependencies" && target.dataset.id) {
    ui.pendingGraphAnchor = null;
    resetGraphUiState();
    ui.clientView.page = "dependency-graph";
    ui.clientView.panels.rightSide = "rail";
    await rpc("setCenterView", { view: "graph" });
    await rpc("centerDependencyGraph", { issueId: target.dataset.id });
    render();
    await loadSelectedIssueDocument();
    render();
    return;
  }
  if (act === "graph-complete") {
    resetGraphUiState();
    await rpc("setDependencyGraphComplete", { complete: true });
    render();
    return;
  }
  if (act === "graph-neighborhood") {
    resetGraphUiState();
    await rpc("setDependencyGraphComplete", { complete: false });
    render();
    return;
  }
  if (act === "graph-more") {
    ui.graphCanvasLimit += 48;
    render();
    return;
  }
  if (act === "graph-list-more") {
    ui.graphListLimit += 50;
    render();
    return;
  }
  if (act === "focus-issue" && target.dataset.id) {
    const enterWorkspace = ui.clientView.page === "focus-workspace"
      || Boolean(target.closest(".issue-card-main, .graph-node-main, .graph-index-main"));
    if (enterWorkspace) enterPrimaryPage("focus-workspace", ui.snapshot);
    ui.clientView.panels.rightSide = "rail";
    await rpc("focusIssue", { issueId: target.dataset.id });
    const run = workspaceRun(ui.snapshot);
    if (enterWorkspace && run && (ui.snapshot.focusedRunId !== run.id || ui.snapshot.workspaceView !== "run")) {
      await rpc("focusRun", { runId: run.id });
    }
    if (enterWorkspace && ui.clientView.page === "focus-workspace") syncReturnPointNavigation();
    await loadSelectedIssueDocument();
    if (mobileClient()) {
      ui.mobileWorkspaceSection = "issue";
      ui.mobileLiveTerminal = false;
    }
    render();
    return;
  }
  if (act === "filter-parent" && target.dataset.id) {
    await rpc("filterParent", { issueId: target.dataset.id });
    render();
    return;
  }
  if (act === "clear-filter") {
    await rpc("clearParentFilter");
    render();
  }
}
