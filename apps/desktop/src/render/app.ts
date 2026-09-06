import { attachTerminal, captureActiveField, centerGraphViewport, dependencyGraphRenderKey, emptyActionAct, emptyActionLabel, languageLabel, paintGraphEdges, pumpMobileOutput, restoreActiveField, restoreGraphAnchor, syncGraphSelection, themeLabel } from "../main";
import { clientCopy } from "../view-helpers";
import { startupCopy } from "../startup-copy";
import type { ScrollPosition } from "../protocol";
import { currentProject, ensureMobileAppearance, focusedRun, mobileClient, mobileMain, mobileNavigation, mobileScopeSheet } from "../view-helpers";
import { escapeHtml } from "../client-utils";
import { hostOverviewPage, keyboardHelpDialog, launchEnvironmentStatus, liftedRunView, projectBlock, quitOfferDialog, runDock, startupSettings, updateDialog, updateSettings, usagePage, viewChangesPanel } from "./shell";
import { issuePanelIcon, projectMain } from "./board";
import { launchForm, loopbackNotice, projectForm, removeDialog } from "./run";
import { panelUiText, refreshPanelSizeFeedback } from "../workbench";
import { ui } from "../ui";

export function render(): void {
  if (!ui.snapshot || !ui.app) return;
  const snap = ui.snapshot;
  const isMobile = mobileClient();
  const activeField = captureActiveField();
  const appearance = isMobile
    ? { ...snap.appearance, ...ensureMobileAppearance() }
    : snap.appearance;
  const copy = isMobile && appearance.language !== snap.appearance.language
    ? clientCopy(appearance.language, snap.copy)
    : snap.copy;
  const { hosts, projects } = snap;
  const project = currentProject(snap);
  document.documentElement.lang = appearance.language === "zh-CN" ? "zh-CN" : "en";
  document.documentElement.dataset.theme = appearance.theme;
  document.documentElement.dataset.mobile = isMobile ? "true" : "false";
  document.title = copy.appName;

  const host = hosts.find((item) => item.id === ui.snapshot?.focusedHostId) ?? hosts[0];
  const empty = ui.snapshot.emptyActions.length > 0;
  const runLifted = !isMobile && snap.workspaceView === "run" && Boolean(focusedRun(snap));
  const showSidebar = !isMobile && ui.sidebarVisible && !runLifted;
  const selectedIssue = snap.board?.selected;
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
  const inspectorOpen = ui.issueDetailVisible && Boolean(selectedIssue);
  const showIssueToggle = !isMobile && Boolean(selectedIssue) && (snap.workspaceView === "project" || runLifted);
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
    !snap.usageOpen &&
    snap.workspaceView === "project" &&
    !runLifted &&
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
    <div class="frame">
      <header class="chrome ${showSidebar ? "with-side" : "side-hidden"}">
        <div class="chrome-lead">
          ${isMobile
            ? `<button type="button" class="chrome-button" data-act="mobile-scope">${escapeHtml(copy.mobileSwitchScope)}</button>`
            : `<button type="button" class="chrome-icon" data-act="toggle-sidebar" aria-label="${escapeHtml(showSidebar ? copy.hideSidebar : copy.showSidebar)}" title="${escapeHtml(showSidebar ? copy.hideSidebar : copy.showSidebar)}">☰</button>
               ${showSidebar ? `<span class="chrome-ui.app">${escapeHtml(copy.appName)}</span>` : ""}`}
        </div>
        <div class="chrome-main">
          <div class="chrome-primary">
            ${!isMobile && !empty && !snap.usageOpen && snap.workspaceView === "project" && !runLifted
              ? `<div class="view-switch" role="tablist">
                  <button type="button" class="${snap.centerView === "board" ? "active" : ""}" data-act="center-view" data-id="board">${escapeHtml(copy.viewBoard)}</button>
                  <button type="button" class="${snap.centerView === "graph" ? "active" : ""}" data-act="center-view" data-id="graph">${escapeHtml(copy.viewGraph)}</button>
                </div>`
              : ""}
            ${runLifted ? `<button type="button" class="chrome-button" data-act="return-board">← ${escapeHtml(copy.returnToBoard)}</button>` : ""}
            ${!isMobile && snap.workspaceView === "host-overview" ? `<span class="chrome-title">${escapeHtml(copy.hostOverview)}</span>` : ""}
            ${!isMobile && snap.usageOpen ? `<span class="chrome-title">${escapeHtml(copy.usage)}</span>` : ""}
            ${!isMobile && !showSidebar ? `<button type="button" class="chrome-button ${snap.workspaceView === "host-overview" ? "active" : ""}" data-act="open-overview">${escapeHtml(copy.hostOverview)}</button>` : ""}
          </div>
          ${!isMobile && project ? `<span class="chrome-context">${escapeHtml(host?.displayName ?? "")} · ${escapeHtml(project.name)}</span>` : ""}
          <div class="chrome-trail">
            ${!isMobile && focusedRun(snap) && !ui.terminalPanelVisible
              ? `<button type="button" class="chrome-button" data-act="show-terminal">${escapeHtml(panelUiText().showTerminal)}</button>`
              : ""}
            ${showIssueToggle
              ? `<button type="button" class="chrome-icon ${inspectorOpen ? "active" : ""}" data-act="toggle-issue" aria-label="${escapeHtml(inspectorOpen ? copy.hideIssueDetail : copy.showIssueDetail)}" title="${escapeHtml(inspectorOpen ? copy.hideIssueDetail : copy.showIssueDetail)}">${issuePanelIcon(inspectorOpen)}</button>`
              : ""}
            <button type="button" class="chrome-button" data-act="settings">${escapeHtml(copy.settings)}</button>
            <button type="button" class="chrome-button ${appearance.theme !== "plain-night" ? "active" : ""}" data-act="shade" data-id="light">${escapeHtml(copy.shadeLight)}</button>
            <button type="button" class="chrome-button ${appearance.theme === "plain-night" ? "active" : ""}" data-act="shade" data-id="dark">${escapeHtml(copy.shadeDark)}</button>
          </div>
        </div>
      </header>
      <div class="body ${showSidebar ? "" : "side-collapsed"}">
        ${showSidebar ? `<aside class="side">
          <div>
            <div class="group-name">${escapeHtml(copy.hosts)}</div>
            ${
              host
                ? `<div class="host-line">
                    <button type="button" class="item active" data-act="toggle-hosts"><span class="dot"></span>${escapeHtml(host.displayName)}${host.local ? `<span class="tag">${escapeHtml(copy.thisMachine)}</span>` : ""}</button>
                    <button type="button" class="title-icon" data-act="pair" aria-label="${escapeHtml(copy.pairAnotherHost)}">⊕</button>
                  </div>
                  <button type="button" class="item ${snap.workspaceView === "host-overview" ? "active" : ""}" data-act="open-overview">${escapeHtml(copy.hostOverview)}</button>
                  <button type="button" class="item ${snap.usageOpen ? "active" : ""}" data-act="open-usage">${escapeHtml(copy.usage)}</button>`
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
            empty
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
              : snap.usageOpen
                ? usagePage(copy, snap)
                : isMobile
                  ? mobileMain(copy, snap)
                  : snap.workspaceView === "host-overview"
                    ? hostOverviewPage(copy, snap)
                    : runLifted
                      ? liftedRunView(copy, snap)
                      : `${projectMain(copy, snap, reuseGraphCanvas)}${runDock(copy, snap)}`
          }
        </main>
      </div>
      ${isMobile && !empty && !snap.usageOpen ? mobileNavigation(copy, snap) : ""}
    </div>
    ${isMobile && ui.mobileScopeOpen ? mobileScopeSheet(copy, snap) : ""}
    ${
      ui.settingsOpen
        ? `<div class="overlay" data-act="close-settings">
            <div class="sheet" data-stop="true">
              <h2>${escapeHtml(copy.settings)}</h2>
              <div class="field">
                <div class="label">${escapeHtml(copy.language)}</div>
                <div class="choices">
                  ${appearance.languages
                    .map(
                      (language) =>
                        `<button type="button" class="${appearance.language === language ? "active" : ""}" data-act="language" data-id="${language}">${escapeHtml(languageLabel(copy, language))}</button>`,
                    )
                    .join("")}
                </div>
              </div>
              <div class="field">
                <div class="label">${escapeHtml(copy.theme)}</div>
                <div class="choices">
                  ${appearance.themes
                    .map(
                      (theme) =>
                        `<button type="button" class="${appearance.theme === theme ? "active" : ""}" data-act="theme" data-id="${theme}">${escapeHtml(themeLabel(copy, theme))}</button>`,
                    )
                    .join("")}
                </div>
              </div>
              ${startupSettings(startupCopy(appearance.language), snap)}
              <div class="field">
                <button type="button" data-act="refresh-launch-environment" ${snap.hostMode === "client-only" ? "disabled" : ""}>${escapeHtml(startupCopy(appearance.language).rereadLaunchEnvironment)}</button>
                ${launchEnvironmentStatus(startupCopy(appearance.language))}
              </div>
              ${updateSettings(copy)}
              <div class="field">
                <label class="label" for="refresh-interval">${escapeHtml(copy.refreshInterval)}</label>
                <input id="refresh-interval" type="number" min="15" step="15" data-field="refreshInterval" value="${Math.round((snap.refreshIntervalMs ?? 300_000) / 1000)}" />
                <p class="hint">${escapeHtml(copy.refreshIntervalHelp)}</p>
              </div>
              <div class="field">
                <label class="label" for="recent-limit">${escapeHtml(copy.recentLimit)}</label>
                <input id="recent-limit" type="number" min="1" max="50" data-field="recentLimit" value="${snap.recentCompletedLimit}" />
                <p class="hint">${escapeHtml(copy.recentLimitHelp)}</p>
              </div>
              <label class="graph-opt">
                <input type="checkbox" data-field="commandPreview" ${snap.showCommandPreview ? "checked" : ""} />
                ${escapeHtml(copy.showCommandPreview)}
              </label>
              ${isMobile ? "" : `<label class="graph-opt">
                <input type="checkbox" data-field="notifyDesktop" ${snap.notifyDesktop ? "checked" : ""} />
                ${escapeHtml(copy.notifyDesktop)}
              </label>
              <label class="graph-opt">
                <input type="checkbox" data-field="notifySound" ${snap.notifySound ? "checked" : ""} />
                ${escapeHtml(copy.notifySound)}
              </label>
              <label class="graph-opt">
                <input type="checkbox" data-field="hostAutoAdvance" ${snap.autoAdvance ? "checked" : ""} />
                ${escapeHtml(copy.autoAdvance)}
              </label>
              <p class="hint">${escapeHtml(copy.autoAdvanceHelp)}</p>
              ${
                currentProject(snap)
                  ? `<label class="graph-opt">
                <input type="checkbox" data-field="projectAutoAdvance" ${currentProject(snap)?.autoAdvance ? "checked" : ""} />
                ${escapeHtml(copy.projectAutoAdvance)}
              </label>
              <label class="graph-opt">
                <input type="checkbox" data-field="restoreAutoAdvance" ${currentProject(snap)?.restoreAutoAdvance ? "checked" : ""} />
                ${escapeHtml(copy.restoreAutoAdvance)}
              </label>
              <div class="field">
                <label class="label" for="restore-delay">${escapeHtml(copy.restoreDelay)}</label>
                <input id="restore-delay" type="number" min="0" max="600" data-field="restoreDelay" value="${Math.round((currentProject(snap)?.restoreDelayMs ?? 60000) / 1000)}" />
              </div>`
                  : ""
              }
              <button type="button" data-act="quit">${escapeHtml(copy.quitHost)}</button>`}
            </div>
          </div>`
        : ""
    }
    ${
      ui.pairingOpen
        ? `<div class="overlay" data-act="close-pairing">
            <div class="sheet pairing-sheet" data-act="pairing-noop">
              <h2>${escapeHtml(copy.pairingTitle)}</h2>
              <p class="hint">${escapeHtml(copy.pairingSamePayload)}</p>
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
    ${ui.changesOpen ? viewChangesPanel(copy) : ""}
    ${ui.keyboardHelpOpen ? keyboardHelpDialog(copy) : ""}
  `;
  refreshPanelSizeFeedback();
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
}
