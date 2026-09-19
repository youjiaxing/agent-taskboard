import { attachTerminal, captureActiveField, centerGraphViewport, dependencyGraphRenderKey, emptyActionAct, emptyActionLabel, paintGraphEdges, pumpMobileOutput, restoreActiveField, restoreGraphAnchor, syncGraphSelection } from "../main";
import { clientCopy } from "../view-helpers";
import { appearancePreferenceLabel, startupCopy } from "../startup-copy";
import type { ScrollPosition } from "../protocol";
import {
  APPEARANCE_DISPLAY_ORDER,
  APPEARANCE_PREFERENCES,
  browserClient,
  ensureBrowserAppearance,
  focusedRun,
  mobileClient,
  mobileMain,
  mobileNavigation,
  mobileScopeSheet,
  resolveTheme,
  viewportClass,
} from "../view-helpers";
import { escapeHtml } from "../client-utils";
import { hostOverviewPage, keyboardHelpDialog, liftedRunView, projectBlock, quitOfferDialog, runDock, settingsPage, updateDialog, usagePage, viewChangesPanel } from "./shell";
import { issuePanelIcon, projectMain } from "./board";
import { launchForm, loopbackNotice, projectForm, removeDialog } from "./run";
import { applyClientPanelWidths, fixedPanelResizeHandle, panelUiText } from "../workbench";
import { ui } from "../ui";
import { formFeedback } from "../form-keys";
import { scheduleEditMenuContextSync } from "../edit-menu";

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
  const runLifted = !isMobile && snap.workspaceView === "run" && Boolean(focusedRun(snap));
  const showSidebar = !isMobile && ui.clientView.panels.sidebarVisible;
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
  const showChangesPanel = !isMobile
    && ui.clientView.page !== "settings"
    && ui.clientView.panels.rightSide === "changes"
    && Boolean(focusedRun(snap));
  const showIssueToggle = !isMobile
    && Boolean(selectedIssue)
    && ["board", "dependency-graph", "focus-workspace"].includes(ui.clientView.page);
  const previousGraphCanvas = ui.app.querySelector<HTMLElement>(".graph-canvas");
  const previousLaunchScrollTop = ui.app.querySelector<HTMLElement>(".launch-sheet")?.scrollTop ?? 0;
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
            ${!isMobile && focusedRun(snap) && ui.clientView.page !== "settings"
              ? `<button type="button" class="chrome-button ${ui.clientView.panels.rightSide === "changes" ? "active" : ""}" data-act="view-changes" data-id="${escapeHtml(focusedRun(snap)?.id ?? "")}" data-global-action="changes">${escapeHtml(copy.viewChanges)}</button>`
              : ""}
            <div class="appearance-menu-wrap">
              <button type="button" class="chrome-button" data-act="appearance-menu" data-global-action="appearance" aria-haspopup="menu" aria-expanded="${ui.appearanceMenuOpen}">${escapeHtml(localCopy.appearance)}</button>
              ${ui.appearanceMenuOpen
                ? `<div class="appearance-menu" role="menu" aria-label="${escapeHtml(localCopy.appearance)}">
                    ${APPEARANCE_DISPLAY_ORDER.map((preference) =>
                      `<button type="button" role="menuitemradio" aria-checked="${appearance.appearancePreference === preference}" class="${appearance.appearancePreference === preference ? "active" : ""}" data-act="appearance" data-id="${preference}">${escapeHtml(appearancePreferenceLabel(localCopy, preference))}</button>`,
                    ).join("")}
                  </div>`
                : ""}
            </div>
            <button type="button" class="chrome-button ${ui.clientView.page === "settings" ? "active" : ""}" data-act="settings" data-global-action="settings">${escapeHtml(copy.settings)}</button>
            <div class="more-menu-wrap">
              <button type="button" class="chrome-icon" data-act="more-menu" data-global-action="more" aria-haspopup="menu" aria-expanded="${ui.moreMenuOpen}" aria-label="${escapeHtml(localCopy.more)}">•••</button>
              ${ui.moreMenuOpen
                ? `<div class="more-menu" role="menu">
                    ${focusedRun(snap) && !ui.terminalPanelVisible ? `<button type="button" role="menuitem" data-act="show-terminal">${escapeHtml(panelUiText().showTerminal)}</button>` : ""}
                    <button type="button" role="menuitem" data-act="keyboard-help">${escapeHtml(copy.keyboardHelp)}</button>
                  </div>`
                : ""}
            </div>
          </div>
        </div>
      </header>
      <div class="body ${showSidebar ? "" : "side-collapsed"}${showChangesPanel ? " changes-open" : ""}">
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
        <main class="workspace ${empty ? "" : "board-open"}${!snap.usageOpen && snap.workspaceView === "project" && focusedRun(snap) ? " has-run" : ""}">
          ${
            ui.clientView.page === "settings"
              ? settingsPage(copy, localCopy, snap, appearance, isMobile)
              : empty
                ? `<div class="empty">
                    ${loopbackNotice(snap.loopbackPage)}
                    <h1>${escapeHtml(copy.noProjectTitle)}</h1>
                    <p>${escapeHtml(copy.noProjectBody)}</p>
                    <div class="actions">
                      ${snap.emptyActions
                        .map(
                          (action, index) =>
                            `<button type="button" class="${index === 0 ? "primary" : ""}" data-act="${emptyActionAct(action)}">${escapeHtml(emptyActionLabel(copy, action))}</button>`,
                        )
                        .join("")}
                    </div>
                  </div>`
                : ui.clientView.page === "usage"
                  ? usagePage(copy, snap)
                  : isMobile
                    ? mobileMain(copy, snap)
                    : ui.clientView.page === "host-overview"
                      ? hostOverviewPage(copy, snap)
                      : ui.clientView.page === "focus-workspace" && runLifted
                        ? liftedRunView(copy, snap)
                        : `${projectMain(copy, snap, reuseGraphCanvas)}${runDock(copy, snap)}`
          }
        </main>
        ${showChangesPanel ? viewChangesPanel(copy) : ""}
      </div>
      ${isMobile && !empty && !["settings", "usage", "host-overview"].includes(ui.clientView.page) ? mobileNavigation(copy, snap) : ""}
    </div>
    ${isMobile && ui.mobileScopeOpen ? mobileScopeSheet(copy, snap) : ""}
    ${
      ui.pairingOpen
        ? `<div class="overlay" data-act="close-pairing">
            <div class="sheet pairing-sheet" data-act="pairing-noop">
              <h2>${escapeHtml(copy.pairingTitle)}</h2>
              <p class="hint">${escapeHtml(copy.pairingSamePayload)}</p>
              <fieldset class="pairing-fields" ${ui.formOperations.pending.has("pairing") ? 'disabled aria-busy="true"' : ""}>
              <div class="field">
                <div class="label">${escapeHtml(copy.pairingThisHost)}</div>
                <label class="label" for="pairing-address">${escapeHtml(copy.pairingAddress)}</label>
                <input id="pairing-address" data-field="address" value="${escapeHtml(ui.pairingAddress)}" />
                <div class="actions">
                  <button type="button" class="primary" data-act="show-offer">${escapeHtml(copy.pairingShow)}</button>
                </div>
                ${
                  ui.snapshot.pairingOffer
                    ? `<div class="offer">
                        <div class="qr">${ui.snapshot.pairingOffer.qrSvg}</div>
                        <pre class="payload">${escapeHtml(ui.snapshot.pairingOffer.text)}</pre>
                        <button type="button" data-act="copy-offer">${escapeHtml(copy.pairingCopy)}</button>
                      </div>`
                    : ""
                }
              </div>
              <div class="field">
                <div class="label">${escapeHtml(copy.pairedClients)}</div>
                ${
                  ui.snapshot.pairedClients.length
                    ? ui.snapshot.pairedClients
                        .map(
                          (client) =>
                            `<div class="client-row"><span>${escapeHtml(client.name)}</span><button type="button" data-act="revoke" data-id="${escapeHtml(client.id)}">${escapeHtml(copy.revokeClient)}</button></div>`,
                        )
                        .join("")
                    : `<div class="nested">${escapeHtml(copy.noPairedClients)}</div>`
                }
              </div>
              <div class="field">
                <div class="label">${escapeHtml(copy.pairingToAnother)}</div>
                <textarea data-field="paste" rows="4" placeholder="${escapeHtml(copy.pairingPaste)}">${escapeHtml(ui.pairingPaste)}</textarea>
                <div class="actions">
                  <button type="button" class="primary" data-act="connect-host">${escapeHtml(copy.pairingConnect)}</button>
                </div>
              </div>
              </fieldset>
              ${ui.formOperations.pending.has("pairing") ? `<p role="status">${escapeHtml(copy.operationPending)}</p>` : ""}
              ${formFeedback("pairing")}
              ${ui.pairingError ? `<p class="notice">${escapeHtml(ui.pairingError)}</p>` : ""}
            </div>
          </div>`
        : ""
    }
    ${ui.formOpen ? projectForm(copy) : ""}
    ${snap.launchForm ? launchForm(copy, snap) : ""}
    ${ui.removeProject ? removeDialog(copy, ui.removeProject) : ""}
    ${snap.quitOffer ? quitOfferDialog(copy) : ""}
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
  const nextLaunchSheet = ui.app.querySelector<HTMLElement>(".launch-sheet");
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
