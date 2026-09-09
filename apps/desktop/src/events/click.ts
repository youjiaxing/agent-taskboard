import { captureGraphAnchor, eventsNeedFullRender, paintGraphEdges, renderStatusBarsOnly, reportClientView, restoreGraphAnchor } from "../main";
import { effectiveClientLanguage, resetGraphUiState } from "../view-helpers";
import type { CenterView, Language, RpcResult, Snapshot, Theme } from "../protocol";
import { checkForUpdates, chooseProjectDirectory, expectedOpening, inferFromLocalPath, installPendingUpdate, loadStartupSettings, openExternalUrl, setHostMode, supersedeProjectInference, syncLaunchDraft } from "../launch-session";
import { clearFormOperation, editableIssueBody, editableIssueRelations, issueBlockersFormKey, issueCreateFormKey, issueEditFormKey, issueOpenFormKey, runFormOperation, usageCustomFormKey } from "../form-keys";
import { ensureMobileAppearance, focusedRun, mobileClient, saveMobileAppearance } from "../view-helpers";
import { inspectorAnchorForIssue, panelIsFloating, positionInspectorAwayFromCard, setPanelFloating, workbenchPanelId } from "../workbench";
import { loadSelectedIssueDocument, loadViewChanges, rpc, rpcDetached } from "../rpc";
import { parsePairingPayload, safeHttpUrl } from "../client-utils";
import { render } from "../render/app";
import { emptyDraft, ui } from "../ui";

let deferredLaunchDiscoverySequence = 0;

async function completeDeferredLaunchDiscovery(
  projectId: string,
  issueId: string,
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
    // The static launch form remains usable when option discovery fails.
  }
}

export async function handleAppClick(event: MouseEvent): Promise<void> {
  const target = (event.target as HTMLElement).closest<HTMLElement>("[data-act]");
  if (!target || !ui.snapshot) return;
  if (target.dataset.stop) event.stopPropagation();
  const act = target.dataset.act;
  if (act === "mobile-scope") {
    ui.mobileScopeOpen = true;
    render();
    return;
  }
  if (act === "close-mobile-scope" && event.target === target) {
    ui.mobileScopeOpen = false;
    render();
    return;
  }
  if (act === "mobile-board") {
    ui.mobileView = "board";
    ui.mobileLiveTerminal = false;
    render();
    return;
  }
  if (act === "mobile-issue") {
    ui.mobileView = "issue";
    ui.mobileLiveTerminal = false;
    render();
    return;
  }
  if (act === "mobile-run") {
    if (focusedRun(ui.snapshot)) ui.mobileView = "run";
    render();
    return;
  }
  if (act === "mobile-live-terminal") {
    ui.mobileLiveTerminal = true;
    render();
    return;
  }
  if (act === "close-settings" && event.target === target) {
    ui.settingsOpen = false;
    render();
    return;
  }
  if (act === "toggle-sidebar") {
    ui.sidebarVisible = !ui.sidebarVisible;
    render();
    return;
  }
  if (act === "toggle-issue") {
    ui.issueDetailVisible = !ui.issueDetailVisible;
    if (ui.issueDetailVisible) {
      ui.frontWorkbenchPanel = "inspector";
    } else if (ui.snapshot.workspaceView === "run") {
      ui.frontWorkbenchPanel = "terminal";
    }
    render();
    return;
  }
  if (act === "panel-mode") {
    const panelId = workbenchPanelId(target.dataset.id);
    if (panelId) setPanelFloating(panelId, !panelIsFloating(panelId));
    return;
  }
  if (act === "hide-terminal") {
    ui.terminalPanelVisible = false;
    if (ui.snapshot.workspaceView === "run") {
      ui.sidebarVisible = ui.sidebarBeforeLift;
      await rpc("returnToBoard");
    }
    render();
    return;
  }
  if (act === "show-terminal") {
    ui.terminalPanelVisible = true;
    ui.frontWorkbenchPanel = "terminal";
    render();
    return;
  }
  if (act === "open-overview") {
    ui.sidebarVisible = true;
    await rpc("openHostOverview");
    render();
    return;
  }
  if (act === "return-board") {
    ui.sidebarVisible = ui.sidebarBeforeLift;
    ui.terminalPanelVisible = true;
    await rpc("returnToBoard");
    render();
    return;
  }
  if (act === "settings") {
    ui.settingsOpen = true;
    await loadStartupSettings();
    ui.pairingOpen = false;
    ui.formOpen = null;
    ui.removeProject = null;
    ui.projectMenuId = "";
    render();
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
    await checkForUpdates(true);
    return;
  }
  if (act === "install-update") {
    await installPendingUpdate();
    return;
  }
  if (act === "update-later") {
    if (event.target !== target && target.closest(".sheet")) return;
    ui.updateState = { kind: "idle" };
    render();
    return;
  }
  if (act === "pairing-noop") {
    return;
  }
  if (act === "close-pairing" && event.target === target) {
    ui.pairingOpen = false;
    ui.pairingError = "";
    render();
    return;
  }
  if (act === "pair") {
    ui.mobileScopeOpen = false;
    ui.pairingOpen = true;
    ui.settingsOpen = false;
    ui.hostPickerOpen = false;
    ui.formOpen = null;
    ui.removeProject = null;
    ui.projectMenuId = "";
    ui.pairingError = "";
    render();
    return;
  }
  if (act === "register") {
    ui.mobileScopeOpen = false;
    ui.formOpen = "register";
    ui.formProjectId = "";
    ui.formDraft = emptyDraft();
    ui.autoFilledProjectName = "";
    supersedeProjectInference();
    ui.formError = "";
    ui.removeError = "";
    ui.projectMenuId = "";
    ui.pairingOpen = false;
    ui.settingsOpen = false;
    ui.removeProject = null;
    render();
    return;
  }
  if (act === "new-issue" && target.dataset.id) {
    ui.createIssueProjectId = target.dataset.id;
    ui.createIssueDraft = { title: "", body: "" };
    ui.createIssueOpen = true;
    clearFormOperation(issueCreateFormKey(ui.createIssueProjectId));
    ui.issueEditOpenId = null;
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
  if (act === "form-noop") {
    return;
  }
  if (act === "keyboard-help") {
    ui.keyboardHelpOpen = true;
    render();
    return;
  }
  if (act === "close-keyboard-help") {
    ui.keyboardHelpOpen = false;
    render();
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
    ui.issueEditDrafts.set(issue.id, { title: current.title, body: editableIssueBody(current) });
    ui.issueEditOpenId = issue.id;
    clearFormOperation(issueEditFormKey(issue.id));
    render();
    ui.app.querySelector<HTMLInputElement>("#issue-edit-title")?.focus();
    return;
  }
  if (act === "cancel-edit-issue" && target.dataset.id) {
    if (ui.formOperations.pending.has(issueEditFormKey(target.dataset.id))) return;
    ui.issueEditOpenId = null;
    render();
    return;
  }
  if (act === "clear-issue-blockers" && target.dataset.id) {
    const issue = ui.snapshot.board?.selected?.id === target.dataset.id ? ui.snapshot.board.selected : null;
    if (!issue || ui.formOperations.pending.has(issueBlockersFormKey(issue.id))) return;
    const current = editableIssueRelations(issue);
    ui.issueRelationDrafts.set(issue.id, { ...current, blockedBy: [] });
    render();
    return;
  }
  if (act === "toggle-issue-open" && target.dataset.id) {
    const issue = ui.snapshot.board?.selected?.id === target.dataset.id ? ui.snapshot.board.selected : null;
    if (!issue) return;
    const key = issueOpenFormKey(issue.id);
    await runFormOperation(key, async () => {
      await rpc("setIssueOpen", { issueId: issue.id, open: !issue.open });
    });
    return;
  }
  if (act === "close-form" && (event.target === target || target.tagName === "BUTTON")) {
    if (ui.projectOperation) return;
    if (target.tagName !== "BUTTON") return;
    ui.formOpen = null;
    supersedeProjectInference();
    ui.formError = "";
    render();
    return;
  }
  if (act === "project-menu" && target.dataset.id) {
    ui.projectMenuId = ui.projectMenuId === target.dataset.id ? "" : target.dataset.id;
    render();
    return;
  }
  if (act === "focus-project" && target.dataset.id) {
    ui.projectMenuId = "";
    ui.mobileScopeOpen = false;
    ui.mobileView = "board";
    ui.sidebarVisible = true;
    await rpc("focusProject", { projectId: target.dataset.id });
    await reportClientView();
    render();
    return;
  }
  if (act === "new-run" && target.dataset.id) {
    ui.projectMenuId = "";
    ui.settingsOpen = false;
    ui.pairingOpen = false;
    ui.formOpen = null;
    ui.launchDraft = null;
    await rpc("prepareRunLaunch", {
      projectId: target.dataset.id,
      language: effectiveClientLanguage(),
    });
    render();
    return;
  }
  if (act === "execute-run" && target.dataset.id && ui.snapshot.focusedProjectId) {
    ui.settingsOpen = false;
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
    await rpc("releaseIssue", { issueId: target.dataset.id });
    render();
    return;
  }
  if (act === "close-launch" && (event.target === target || target.tagName === "BUTTON")) {
    await rpc("cancelRunLaunch");
    ui.launchDraft = null;
    ui.launchPickerProjectId = "";
    ui.launchPickerAgentId = "";
    ui.launchPreviewSequence += 1;
    if (ui.launchPreviewTimer != null) window.clearTimeout(ui.launchPreviewTimer);
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
    ui.launchDraft = null;
    await rpc("prepareRunLaunch", {
      projectId: form.projectId,
      issueId: form.issueId,
      agentId: ui.launchPickerAgentId,
      language: effectiveClientLanguage(),
    });
    render();
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
    ui.sidebarBeforeLift = ui.sidebarVisible;
    ui.issueDetailVisible = true;
    ui.terminalPanelVisible = true;
    ui.frontWorkbenchPanel = "terminal";
    await rpc("focusRun", { runId: target.dataset.id });
    await loadSelectedIssueDocument();
    if (mobileClient()) {
      ui.mobileView = "run";
      ui.mobileLiveTerminal = false;
    } else {
      ui.sidebarVisible = false;
    }
    render();
    return;
  }
  if (act === "open-usage") {
    ui.mobileScopeOpen = false;
    ui.settingsOpen = false;
    ui.pairingOpen = false;
    ui.formOpen = null;
    ui.frontWorkbenchPanel = "usage";
    await rpc("openUsage");
    render();
    return;
  }
  if (act === "close-usage") {
    await rpc("closeUsage");
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
    await rpc("openUsageForRun", { runId: target.dataset.id });
    render();
    return;
  }
  if (act === "open-run-usage" && target.dataset.id) {
    ui.terminalPanelVisible = true;
    ui.frontWorkbenchPanel = "terminal";
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
    await rpc("stopRun", { runId: target.dataset.id });
    if (mobileClient()) {
      ui.mobileView = "board";
      ui.mobileLiveTerminal = false;
    }
    render();
    return;
  }
  if (act === "view-changes" && target.dataset.id) {
    ui.changesOpen = true;
    ui.changesScope = "this-round";
    ui.noteTarget = null;
    ui.noteDraft = "";
    await loadViewChanges(target.dataset.id, ui.changesScope);
    render();
    return;
  }
  if (act === "close-changes") {
    ui.changesOpen = false;
    ui.changesView = null;
    ui.noteTarget = null;
    ui.noteDraft = "";
    render();
    return;
  }
  if (act === "changes-scope" && target.dataset.id) {
    const scope = target.dataset.id === "uncommitted" ? "uncommitted" : "this-round";
    ui.changesScope = scope;
    const runId = ui.changesView?.runId ?? ui.snapshot.focusedRunId;
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
    const runId = ui.changesView?.runId ?? ui.snapshot.focusedRunId;
    if (runId) await loadViewChanges(runId, ui.changesScope);
    render();
    return;
  }
  if (act === "cancel-quit") {
    await rpc("cancelQuit");
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
    ui.mobileScopeOpen = false;
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
    ui.mobileScopeOpen = false;
    ui.removeProject = ui.snapshot.projects.find((item) => item.id === target.dataset.id) ?? null;
    ui.removeError = "";
    ui.projectMenuId = "";
    render();
    return;
  }
  if (act === "close-remove" && (event.target === target || target.tagName === "BUTTON")) {
    if (ui.projectOperation) return;
    ui.removeProject = null;
    ui.removeError = "";
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
    ui.pairingError = "";
    const addressInput = ui.app.querySelector<HTMLInputElement>("[data-field='address']");
    ui.pairingAddress = addressInput?.value ?? ui.pairingAddress;
    try {
      await rpc("beginPairingOffer", { address: ui.pairingAddress });
    } catch (error) {
      ui.pairingError = error instanceof Error ? error.message : String(error);
    }
    render();
    return;
  }
  if (act === "copy-offer" && ui.snapshot.pairingOffer) {
    await navigator.clipboard.writeText(ui.snapshot.pairingOffer.text);
    return;
  }
  if (act === "revoke" && target.dataset.id) {
    ui.pairingError = "";
    try {
      await rpc("revokeClient", { clientId: target.dataset.id });
    } catch (error) {
      ui.pairingError = error instanceof Error ? error.message : String(error);
    }
    render();
    return;
  }
  if (act === "connect-host") {
    ui.pairingError = "";
    const pasteInput = ui.app.querySelector<HTMLTextAreaElement>("[data-field='paste']");
    ui.pairingPaste = pasteInput?.value ?? ui.pairingPaste;
    const parsed = parsePairingPayload(ui.pairingPaste);
    if (!parsed) {
      ui.pairingError = ui.snapshot.copy.pairingPaste;
      render();
      return;
    }
    try {
      await rpc("pairRemoteHost", parsed);
      ui.pairingPaste = "";
      ui.pairingOpen = false;
    } catch (error) {
      ui.pairingError = error instanceof Error ? error.message : String(error);
    }
    render();
    return;
  }
  if (act === "language" && target.dataset.id) {
    if (mobileClient()) {
      const appearance = ensureMobileAppearance();
      saveMobileAppearance({ ...appearance, language: target.dataset.id as Language });
    } else {
      await rpc("setLanguage", { language: target.dataset.id });
    }
    render();
    return;
  }
  if (act === "theme" && target.dataset.id) {
    if (mobileClient()) {
      const appearance = ensureMobileAppearance();
      const theme = target.dataset.id as Theme;
      saveMobileAppearance({
        ...appearance,
        theme,
        lastLightTheme: theme === "plain-night" ? appearance.lastLightTheme : theme,
      });
    } else {
      await rpc("setTheme", { theme: target.dataset.id });
    }
    render();
    return;
  }
  if (act === "shade") {
    const current = mobileClient() ? ensureMobileAppearance() : ui.snapshot.appearance;
    const next = target.dataset.id === "dark" ? "plain-night" : current.lastLightTheme;
    if (mobileClient()) {
      saveMobileAppearance({ ...current, theme: next });
    } else {
      await rpc("setTheme", { theme: next });
    }
    render();
    return;
  }
  if (act === "quit") {
    ui.settingsOpen = false;
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
    ui.issueDetailVisible = true;
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
    ui.issueDetailVisible = true;
    ui.inspectorAnchorIssueId = target.dataset.id;
    await rpc("focusIssue", { issueId: target.dataset.id });
    render();
    await loadSelectedIssueDocument();
    if (mobileClient()) {
      ui.mobileView = "issue";
      ui.mobileLiveTerminal = false;
    } else if (target.closest(".issue-card") && ui.snapshot.focusedRunId) {
      ui.sidebarBeforeLift = ui.sidebarVisible;
      await rpc("focusRun", { runId: ui.snapshot.focusedRunId });
      ui.sidebarVisible = false;
    }
    render();
    positionInspectorAwayFromCard(inspectorAnchorForIssue(ui.inspectorAnchorIssueId));
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
