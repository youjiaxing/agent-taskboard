import { attachTerminal, captureActiveField, centerGraphViewport, dependencyGraphRenderKey, emptyActionAct, emptyActionLabel, paintGraphEdges, pumpMobileOutput, restoreActiveField, syncGraphSelection } from "../main";
import { clientCopy, restoreGraphAnchor } from "../view-helpers";
import { appearancePreferenceLabel, startupCopy } from "../startup-copy";
import type { ScrollPosition } from "../protocol";
import {
  APPEARANCE_DISPLAY_ORDER,
  APPEARANCE_PREFERENCES,
  browserClient,
  ensureBrowserAppearance,
  focusedRun,
  mobileClient,
  workspaceRun,
  mobileMain,
  mobileNavigation,
  mobileScopeSheet,
  resolveTheme,
  viewportClass,
} from "../view-helpers";
import { escapeHtml } from "../client-utils";
import { dangerConfirmationDialog, focusWorkspaceView, hostOverviewPage, keyboardHelpDialog, projectBlock, quitOfferDialog, runDock, settingsPage, updateDialog, usagePage } from "./shell";
import { issuePanelIcon, projectMain } from "./board";
import { launchForm, loopbackNotice, projectForm, removeDialog } from "./run";
import { applyClientPanelWidths, fixedPanelResizeHandle } from "../workbench";
import { ui } from "../ui";
import { formFeedback } from "../form-keys";
import { scheduleEditMenuContextSync } from "../edit-menu";
import { dialog, dialogDismissButton } from "../components/dialog";
import { button, emptyState, formField, menu, notice, textArea, textInput } from "../components/primitives";
import { syncDialogFocus } from "../components/dialog-controller";

function pairingDialog(copy: import("../protocol").ShellCopy, localCopy: ReturnType<typeof startupCopy>, snap: import("../protocol").Snapshot): string {
  const pending = ui.formOperations.pending.has("pairing");
  const body = `<p class="hint">${escapeHtml(copy.pairingSamePayload)}</p>
    <fieldset class="pairing-fields" ${pending ? 'disabled aria-busy="true"' : ""}>
      ${formField({
        id: "pairing-address",
        label: `${copy.pairingThisHost} · ${copy.pairingAddress}`,
        control: `${textInput({ id: "pairing-address", value: ui.pairingAddress, attributes: { "data-field": "address" } })}<div class="actions">${button({ id: "show-offer", label: copy.pairingShow }, { variant: "primary" })}</div>`,
      })}
      ${snap.pairingOffer ? `<div class="offer"><div class="qr">${snap.pairingOffer.qrSvg}</div><pre class="payload">${escapeHtml(snap.pairingOffer.text)}</pre>${button({ id: "copy-offer", label: copy.pairingCopy })}</div>` : ""}
      ${formField({
        label: copy.pairingToAnother,
        control: `${textArea({ value: ui.pairingPaste, rows: 4, placeholder: copy.pairingPaste, attributes: { "data-field": "paste" } })}<div class="actions">${button({ id: "connect-host", label: copy.pairingConnect }, { variant: "primary" })}</div>`,
      })}
    </fieldset>
    ${pending ? `<p role="status">${escapeHtml(copy.operationPending)}</p>` : ""}
    ${formFeedback("pairing")}
    ${ui.pairingError ? notice({ status: "danger", role: "alert", message: ui.pairingError }) : ""}`;
  return dialog({
    id: "pairing",
    tier: "form",
    title: copy.pairingTitle,
    body,
    closeLabel: localCopy.close,
    dismissible: !pending,
    initialFocus: "first-field",
    className: "pairing-sheet",
    actions: dialogDismissButton(copy.cancel, { disabled: pending }),
  });
}

export function render(): void {
  if (!ui.snapshot || !ui.app) return;
  const snap = ui.snapshot;
  const isMobile = mobileClient();
  const activeField = captureActiveField();
  const browserAppearance = browserClient() ? ensureBrowserAppearance() : null;
  const appearance = {
    ...snap.appearance,
    language: isMobile && browserAppearance ? browserAppearance.language : snap.appearance.language,
    appearancePreference: browserAppearance?.appearancePreference ?? snap.appearance.appearancePreference,
    appearancePreferences: browserAppearance ? APPEARANCE_PREFERENCES : snap.appearance.appearancePreferences,
  };
  const copy = appearance.language !== snap.appearance.language
    ? clientCopy(appearance.language, snap.copy)
    : snap.copy;
  const localCopy = startupCopy(appearance.language);
  const resolvedTheme = resolveTheme(appearance.appearancePreference, ui.systemAppearance);
  const { hosts, projects } = snap;
  document.documentElement.lang = appearance.language === "zh-CN" ? "zh-CN" : "en";
  document.documentElement.dataset.theme = resolvedTheme;
  document.documentElement.dataset.mobile = isMobile ? "true" : "false";
  document.documentElement.dataset.viewport = viewportClass();
  document.title = copy.appName;

  const host = hosts.find((item) => item.id === ui.snapshot?.focusedHostId) ?? hosts[0];
  const empty = ui.snapshot.emptyActions.length > 0;
  const runWindow = Boolean(ui.nativeRunWindowRunId);
  const focusRun = workspaceRun(snap);
  const showSidebar = !isMobile && !runWindow && ui.clientView.panels.sidebarVisible;
  const selectedIssue = snap.board?.selected;
  const primaryIdentity = ui.clientView.page === "settings"
    ? copy.settings
    : ui.clientView.page === "host-overview"
      ? copy.hostOverview
      : ui.clientView.page === "usage"
        ? copy.usage
        : ui.clientView.page === "focus-workspace"
          ? selectedIssue?.title ?? focusedRun(snap)?.agentName ?? copy.appName
          : host?.displayName ?? copy.appName;
  const showReturn = Boolean(ui.clientView.returnPoint && ui.clientView.page !== ui.clientView.returnPoint.page);
  const previousDetailScrollNode = ui.app.querySelector<HTMLElement>(".detail-scroll");
  if (previousDetailScrollNode && ui.renderedDetailIssueId) {
    ui.issueDetailScrollPositions.set(ui.renderedDetailIssueId, {
      scrollTop: previousDetailScrollNode.scrollTop,
      scrollLeft: previousDetailScrollNode.scrollLeft,
    });
  }
  const previousLanes = ui.app.querySelector<HTMLElement>(".lanes");
  if (previousLanes && ui.renderedBoardProjectId) {
    const laneScrolls: Record<string, ScrollPosition> = {};
    for (const lane of previousLanes.querySelectorAll<HTMLElement>(".lane[data-lane]")) {
      const key = lane.dataset.lane;
      if (!key) continue;
      laneScrolls[key] = { scrollTop: lane.scrollTop, scrollLeft: lane.scrollLeft };
    }
    ui.boardScrollPositions.set(ui.renderedBoardProjectId, {
      scrollTop: previousLanes.scrollTop,
      scrollLeft: previousLanes.scrollLeft,
      lanes: laneScrolls,
    });
  }
  const previousWorkspace = ui.app.querySelector<HTMLElement>(".workspace");
  if (previousWorkspace && ui.renderedMobileWorkspaceKey) {
    ui.mobileWorkspaceScrollPositions.set(ui.renderedMobileWorkspaceKey, {
      scrollTop: previousWorkspace.scrollTop,
      scrollLeft: previousWorkspace.scrollLeft,
    });
  }
  const inspectorOpen = ui.clientView.panels.rightSide === "rail" && Boolean(selectedIssue);
  const showIssueToggle = !isMobile && !runWindow
    && Boolean(selectedIssue)
    && ["board", "dependency-graph", "focus-workspace"].includes(ui.clientView.page);
  const previousGraphCanvas = ui.app.querySelector<HTMLElement>(".graph-canvas");
  const previousLaunchScrollTop = ui.app.querySelector<HTMLElement>(".launch-sheet .dialog-content")?.scrollTop ?? 0;
  const previousGraph = previousGraphCanvas
    ? {
        canvas: previousGraphCanvas,
        projectId: ui.renderedGraphProjectId,
        centerId: ui.renderedGraphCenterId,
        renderKey: ui.renderedGraphKey,
        scrollLeft: previousGraphCanvas.scrollLeft,
        scrollTop: previousGraphCanvas.scrollTop,
        clientWidth: previousGraphCanvas.clientWidth,
        clientHeight: previousGraphCanvas.clientHeight,
        scrollWidth: previousGraphCanvas.scrollWidth,
        scrollHeight: previousGraphCanvas.scrollHeight,
      }
    : null;
  const desktopProjectGraph =
    !isMobile &&
    !empty &&
    ui.clientView.page === "dependency-graph" &&
    snap.centerView === "graph" &&
    Boolean(snap.board?.graph);
  const nextGraphKey = desktopProjectGraph ? dependencyGraphRenderKey(snap.board) : "";
  const nextGraphCenterId = desktopProjectGraph ? snap.board?.graph?.centerId ?? "" : "";
  const graphContentChanged = Boolean(previousGraph && previousGraph.renderKey !== nextGraphKey);
  const reuseGraphCanvas = Boolean(
    previousGraph &&
      previousGraph.projectId === snap.focusedProjectId &&
      previousGraph.renderKey === nextGraphKey,
  );
  if (!ui.pairingAddress) {
    ui.pairingAddress = (ui.snapshot.loopbackPage.url || "http://127.0.0.1:10529/").replace(/\/$/, "");
  }

  ui.app.innerHTML = `
    <div class="frame page-${ui.clientView.page}">
      <header class="chrome ${showSidebar ? "with-side" : "side-hidden"}">
        <div class="chrome-lead">
          ${isMobile
            ? `<button type="button" class="chrome-icon" data-act="mobile-scope" aria-label="${escapeHtml(copy.mobileSwitchScope)}">☰</button>`
            : `<button type="button" class="chrome-icon" data-act="toggle-sidebar" aria-label="${escapeHtml(showSidebar ? copy.hideSidebar : copy.showSidebar)}" title="${escapeHtml(showSidebar ? copy.hideSidebar : copy.showSidebar)}">☰</button>`}
        </div>
        <div class="chrome-main">
          <div class="chrome-primary">
            ${showReturn ? `<button type="button" class="chrome-button" data-act="return-page">← ${escapeHtml(localCopy.back)}</button>` : ""}
            <span class="chrome-title" data-current-identity>${escapeHtml(primaryIdentity)}</span>
          </div>
          <div class="chrome-trail" data-global-actions>
            ${showIssueToggle
              ? `<button type="button" class="chrome-icon ${inspectorOpen ? "active" : ""}" data-act="toggle-issue" data-global-action="right-rail" aria-label="${escapeHtml(inspectorOpen ? copy.hideIssueDetail : copy.showIssueDetail)}" title="${escapeHtml(inspectorOpen ? copy.hideIssueDetail : copy.showIssueDetail)}">${issuePanelIcon(inspectorOpen)}</button>`
              : ""}
            ${!isMobile && !runWindow && focusRun && ui.clientView.page === "focus-workspace"
              ? `<button type="button" class="chrome-button ${ui.clientView.panels.rightSide === "changes" ? "active" : ""}" data-act="view-changes" data-id="${escapeHtml(focusRun.id)}" data-global-action="changes">${escapeHtml(copy.viewChanges)}</button>`
              : ""}
            <div class="appearance-menu-wrap">
              <button type="button" class="chrome-button" data-act="appearance-menu" data-global-action="appearance" aria-haspopup="menu" aria-expanded="${ui.appearanceMenuOpen}">${escapeHtml(localCopy.appearance)}</button>
              ${ui.appearanceMenuOpen
                ? menu({
                    className: "appearance-menu",
                    label: localCopy.appearance,
                    actions: APPEARANCE_DISPLAY_ORDER.map((preference) => ({
                      id: "appearance",
                      label: appearancePreferenceLabel(localCopy, preference),
                      pressed: appearance.appearancePreference === preference,
                      data: { id: preference },
                    })),
                  })
                : ""}
            </div>
            <button type="button" class="chrome-button ${ui.clientView.page === "settings" ? "active" : ""}" data-act="settings" data-global-action="settings">${escapeHtml(copy.settings)}</button>
            <div class="more-menu-wrap">
              <button type="button" class="chrome-icon" data-act="more-menu" data-global-action="more" aria-haspopup="menu" aria-expanded="${ui.moreMenuOpen}" aria-label="${escapeHtml(localCopy.more)}">•••</button>
              ${ui.moreMenuOpen
                ? menu({
                    className: "more-menu",
                    label: localCopy.more,
                    actions: [{ id: "keyboard-help", label: copy.keyboardHelp }],
                  })
                : ""}
            </div>
          </div>
        </div>
      </header>
      <div class="body ${showSidebar ? "" : "side-collapsed"}">
        ${showSidebar ? `<aside class="side" data-fixed-panel="sidebar">
          ${fixedPanelResizeHandle("sidebar")}
          <div class="host-area">
            ${
              host
                ? `<div class="host-line">
                    <button type="button" class="item active" data-act="toggle-hosts"><span class="dot"></span><span class="host-name">${escapeHtml(host.displayName)}</span>${host.local ? `<span class="tag">${escapeHtml(copy.thisMachine)}</span>` : ""}</button>
                    <button type="button" class="title-icon" data-act="pair" aria-label="${escapeHtml(copy.pairAnotherHost)}">⊕</button>
                  </div>
                  <button type="button" class="item ${ui.clientView.page === "host-overview" ? "active" : ""}" data-act="open-overview">${escapeHtml(copy.hostOverview)}</button>
                  <button type="button" class="item ${ui.clientView.page === "usage" ? "active" : ""}" data-act="open-usage">${escapeHtml(copy.usage)}</button>`
                : ""
            }
            ${
              ui.hostPickerOpen && hosts.length > 1
                ? `<div class="host-picker">${hosts
                    .map(
                      (item) =>
                        `<button type="button" class="item ${item.id === host?.id ? "active" : ""}" data-act="focus-host" data-id="${escapeHtml(item.id)}">${escapeHtml(item.displayName)}${item.local ? `<span class="tag">${escapeHtml(copy.thisMachine)}</span>` : ""}</button>`,
                    )
                    .join("")}</div>`
                : ""
            }
          </div>
          <div>
            <div class="group-head">
              <div class="group-name">${escapeHtml(copy.projects)}</div>
              <button type="button" class="title-icon" data-act="register" aria-label="${escapeHtml(copy.addProject)}">＋</button>
            </div>
            ${
              projects.length
                ? projects
                    .map((project) => projectBlock(copy, snap, project, snap.focusedProjectId))
                    .join("")
                : `<div class="nested">${escapeHtml(copy.noProjectTitle)}</div>`
            }
          </div>
        </aside>` : ""}
        <main class="workspace ${empty ? "" : "board-open"}${ui.clientView.page === "focus-workspace" ? " focus-workspace-open" : ""}${!snap.usageOpen && snap.workspaceView === "project" && focusedRun(snap) ? " has-run" : ""}">
          ${
            ui.clientView.page === "settings"
              ? settingsPage(copy, localCopy, snap, appearance, isMobile)
              : empty
                ? `${loopbackNotice(snap.loopbackPage)}${emptyState({
                    title: copy.noProjectTitle,
                    description: copy.noProjectBody,
                    actions: snap.emptyActions.map((action, index) => button({
                      id: emptyActionAct(action),
                      label: emptyActionLabel(copy, action),
                    }, { variant: index === 0 ? "primary" : "secondary" })).join(""),
                  })}`
                : ui.clientView.page === "usage"
                  ? usagePage(copy, snap)
                  : isMobile
                    ? mobileMain(copy, snap)
                    : ui.clientView.page === "host-overview"
                      ? hostOverviewPage(copy, snap)
                      : ui.clientView.page === "focus-workspace"
                        ? focusWorkspaceView(copy, snap)
                        : `${projectMain(copy, snap, reuseGraphCanvas)}${runDock(copy, snap)}`
          }
        </main>
      </div>
      ${isMobile && !empty && !["settings", "usage", "host-overview"].includes(ui.clientView.page) ? mobileNavigation(copy, snap) : ""}
    </div>
    ${isMobile && ui.mobileScopeOpen ? mobileScopeSheet(copy, snap) : ""}
    ${ui.pairingOpen ? pairingDialog(copy, localCopy, snap) : ""}
    ${ui.formOpen ? projectForm(copy) : ""}
    ${snap.launchForm ? launchForm(copy, snap) : ""}
    ${ui.removeProject ? removeDialog(copy, ui.removeProject) : ""}
    ${snap.quitOffer ? quitOfferDialog(copy) : ""}
    ${dangerConfirmationDialog(copy)}
    ${updateDialog(copy)}
    ${ui.keyboardHelpOpen ? keyboardHelpDialog(copy) : ""}
  `;
  applyClientPanelWidths();
  const graphPlaceholder = ui.app.querySelector<HTMLElement>("[data-preserve-graph-canvas]");
  if (reuseGraphCanvas && previousGraph && graphPlaceholder) {
    graphPlaceholder.replaceWith(previousGraph.canvas);
  }
  const graphCanvas = ui.app.querySelector<HTMLElement>(".graph-canvas");
  const sameGraphCenter = Boolean(
    previousGraph &&
      previousGraph.projectId === snap.focusedProjectId &&
      previousGraph.centerId === nextGraphCenterId,
  );
  if (graphCanvas && sameGraphCenter && !graphContentChanged && previousGraph) {
    graphCanvas.scrollLeft = previousGraph.scrollLeft;
    graphCanvas.scrollTop = previousGraph.scrollTop;
  }
  ui.renderedGraphKey = graphCanvas ? nextGraphKey : "";
  ui.renderedGraphProjectId = graphCanvas ? snap.focusedProjectId : "";
  ui.renderedGraphCenterId = graphCanvas ? nextGraphCenterId : "";
  const graphLayoutChanged = Boolean(
    graphCanvas &&
      previousGraph &&
      (graphCanvas.clientWidth !== previousGraph.clientWidth ||
        graphCanvas.clientHeight !== previousGraph.clientHeight ||
        graphCanvas.scrollWidth !== previousGraph.scrollWidth ||
        graphCanvas.scrollHeight !== previousGraph.scrollHeight),
  );
  if (!reuseGraphCanvas || graphLayoutChanged) {
    paintGraphEdges();
  }
  syncGraphSelection(graphCanvas, snap.board?.selected?.id);
  const restoredGraphAnchor = Boolean(
    graphCanvas && ui.pendingGraphAnchor && restoreGraphAnchor(graphCanvas, ui.pendingGraphAnchor),
  );
  if (graphCanvas && ui.pendingGraphAnchor) ui.pendingGraphAnchor = null;
  if (graphCanvas && (!sameGraphCenter || graphContentChanged) && !restoredGraphAnchor) {
    centerGraphViewport(graphCanvas, nextGraphCenterId);
  }
  if (restoredGraphAnchor) paintGraphEdges();
  restoreActiveField(activeField);
  syncDialogFocus();
  const nextLaunchSheet = ui.app.querySelector<HTMLElement>(".launch-sheet .dialog-content");
  if (nextLaunchSheet) nextLaunchSheet.scrollTop = previousLaunchScrollTop;
  const nextDetailScroll = ui.app.querySelector<HTMLElement>(".detail-scroll");
  const savedDetailScroll = selectedIssue
    ? ui.issueDetailScrollPositions.get(selectedIssue.id)
    : undefined;
  if (nextDetailScroll && savedDetailScroll) {
    nextDetailScroll.scrollTop = savedDetailScroll.scrollTop;
    nextDetailScroll.scrollLeft = savedDetailScroll.scrollLeft;
  }
  const nextLanes = ui.app.querySelector<HTMLElement>(".lanes");
  const savedBoardScroll = ui.boardScrollPositions.get(snap.focusedProjectId);
  if (nextLanes && savedBoardScroll) {
    nextLanes.scrollTop = savedBoardScroll.scrollTop;
    nextLanes.scrollLeft = savedBoardScroll.scrollLeft;
    for (const lane of nextLanes.querySelectorAll<HTMLElement>(".lane[data-lane]")) {
      const key = lane.dataset.lane;
      const position = key ? savedBoardScroll.lanes[key] : undefined;
      if (!position) continue;
      lane.scrollTop = position.scrollTop;
      lane.scrollLeft = position.scrollLeft;
    }
  }
  const nextWorkspace = ui.app.querySelector<HTMLElement>(".workspace");
  const nextMobileWorkspaceKey = isMobile ? `${snap.focusedProjectId}:${ui.mobileView}` : "";
  const savedMobileWorkspaceScroll = ui.mobileWorkspaceScrollPositions.get(nextMobileWorkspaceKey);
  if (nextWorkspace && savedMobileWorkspaceScroll) {
    nextWorkspace.scrollTop = savedMobileWorkspaceScroll.scrollTop;
    nextWorkspace.scrollLeft = savedMobileWorkspaceScroll.scrollLeft;
  }
  ui.renderedDetailIssueId = selectedIssue?.id ?? "";
  ui.renderedBoardProjectId = nextLanes ? snap.focusedProjectId : "";
  ui.renderedMobileWorkspaceKey = nextWorkspace ? nextMobileWorkspaceKey : "";
  if (isMobile && !ui.mobileLiveTerminal) {
    ui.ptyPumping = false;
    void pumpMobileOutput(snap);
  } else {
    ui.mobilePtyPumping = false;
    attachTerminal(snap);
  }
  scheduleEditMenuContextSync();
}
