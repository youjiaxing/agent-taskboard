import { escapeHtml } from "../client-utils";
import { confirmationDialog } from "../components/dialog";
import { button, iconButton, notice, selectControl, type ActionDescriptor, type SelectOption } from "../components/primitives";
import type { RunRestoreResult, RunSummary, ShellCopy, Snapshot } from "../protocol";
import { effectiveClientLanguage } from "../view-helpers";
import { ui } from "../ui";

type OrganizationActionMode = "icons" | "text";

export function runOrganizationLabels() {
  return effectiveClientLanguage() === "zh-CN"
    ? {
        pinnedRuns: "置顶 Run",
        archive: "Run 归档",
        archiveHint: "当前 Host 上已归档的 Run。恢复不会启动进程，也不会自动打开 Run。",
        archiveEmpty: "还没有归档 Run",
        archiveFilteredEmpty: "这个 Project 没有归档 Run",
        archiveLoading: "正在读取归档 Run…",
        projectFilter: "Project",
        allProjects: "全部 Project",
        removedProject: "已移除 Project",
        selectRun: "选择一条归档 Run 查看只读详情",
        recentOutput: "最近输出（只读）",
        telemetry: "Telemetry 摘要",
        telemetryEmpty: "没有 telemetry 数据",
        totalTokens: "总 token",
        archivedAt: "归档于",
        pin: "置顶",
        unpin: "取消置顶",
        archiveRun: "归档",
        restore: "恢复",
        openRun: "打开 Run",
        restored: "Run 已恢复。它仍保持 ended，不会自动恢复置顶或 PTY。",
        retry: "重试",
        restoreProjectTitle: "恢复并重建原 Project？",
        restoreProjectBody: "这条 Run 的原 Project 已从 Host 移除。确认后会按原登记信息重建 Project，再恢复 Run。",
        restoreProjectConfirm: "重建并恢复",
        originalProjectId: "原 Project ID",
        originalProjectName: "原名称",
        originalDirectory: "原目录",
        conflictTitle: "现在不能恢复这条 Run",
        conflictTombstoneMissing: "Host 找不到原 Project 墓碑。",
        conflictDirectoryMissing: "原 Project 目录不存在。",
        conflictDirectoryInUse: "原 Project 目录已被另一 Project 使用。",
        conflictRevisionChanged: "Project 墓碑已经变化，请关闭后重新发起恢复。",
        conflictActiveProject: "同一 Project ID 同时存在登记与墓碑，需要先修复 Host 设置。",
        persistenceRetryAction: "重试刚才的操作",
        unknownProject: "Project",
        close: "关闭",
        cancel: "取消",
      }
    : {
        pinnedRuns: "Runs kept on top",
        archive: "Run archive",
        archiveHint: "Archived Runs on the current Host. Restoring does not start a process or open the Run.",
        archiveEmpty: "No archived Runs yet",
        archiveFilteredEmpty: "No archived Runs for this Project",
        archiveLoading: "Loading archived Runs…",
        projectFilter: "Project",
        allProjects: "All Projects",
        removedProject: "Removed Project",
        selectRun: "Select an archived Run for read-only details",
        recentOutput: "Recent output (read only)",
        telemetry: "Telemetry summary",
        telemetryEmpty: "No telemetry data",
        totalTokens: "Total tokens",
        archivedAt: "Archived",
        pin: "Keep on top",
        unpin: "Remove from top",
        archiveRun: "Archive",
        restore: "Restore",
        openRun: "Open Run",
        restored: "The Run was restored. It remains ended and its pin and PTY were not restored.",
        retry: "Retry",
        restoreProjectTitle: "Restore and recreate the original Project?",
        restoreProjectBody: "The original Project was removed from this Host. Confirm to recreate its registration before restoring the Run.",
        restoreProjectConfirm: "Recreate and restore",
        originalProjectId: "Original Project ID",
        originalProjectName: "Original name",
        originalDirectory: "Original directory",
        conflictTitle: "This Run cannot be restored now",
        conflictTombstoneMissing: "The Host cannot find the original Project tombstone.",
        conflictDirectoryMissing: "The original Project directory is missing.",
        conflictDirectoryInUse: "Another Project already uses the original directory.",
        conflictRevisionChanged: "The Project tombstone changed. Close this dialog and start the restore again.",
        conflictActiveProject: "The same Project ID has both an active registration and a tombstone. Repair the Host settings first.",
        persistenceRetryAction: "Retry the last action",
        unknownProject: "Project",
        close: "Close",
        cancel: "Cancel",
      };
}

export function runOrganizationWritable(snap: Snapshot): boolean {
  return Boolean(snap.capabilities.runOrganization)
    && !runPersistenceWritesBlocked(snap);
}

export function runPersistenceWritesBlocked(snap: Snapshot): boolean {
  return snap.capabilities.runPersistenceWrites === false || Boolean(snap.runPersistenceRecovery);
}

function actionKey(action: "pin" | "archive" | "restore", runId: string): string {
  return `${action}:${runId}`;
}

export function runOrganizationActions(
  snap: Snapshot,
  run: RunSummary,
  mode: OrganizationActionMode = "text",
): string {
  if (!snap.capabilities.runOrganization || run.archivedAtMs) return "";
  const labels = runOrganizationLabels();
  const writable = runOrganizationWritable(snap);
  const disabledReason = writable ? "" : snap.copy.runPersistenceWriteBlocked;
  const pinLabel = run.pinnedAtMs ? labels.unpin : labels.pin;
  const pinPending = ui.runOrganizationPending.has(actionKey("pin", run.id));
  const archivePending = ui.runOrganizationPending.has(actionKey("archive", run.id));
  const pin = {
    id: "set-run-pinned",
    label: pinLabel,
    icon: run.pinnedAtMs ? "↓" : "↑",
    disabled: !writable,
    busy: pinPending,
    pressed: Boolean(run.pinnedAtMs),
    data: { id: run.id, pinned: run.pinnedAtMs ? "false" : "true" },
  };
  const archive = run.status === "ended"
    ? {
        id: "archive-run",
        label: labels.archiveRun,
        icon: "▣",
        disabled: !writable,
        busy: archivePending,
        data: { id: run.id },
      }
    : null;
  const renderAction = (action: ActionDescriptor) => mode === "icons"
    ? iconButton(action, {
        className: "run-row-action",
        attributes: { title: action.label, "aria-pressed": action.pressed, "data-run-organization": action.id, "data-disabled-reason": disabledReason || undefined },
      })
    : button(action, {
        variant: "ghost",
        className: "run-organization-action",
        attributes: { title: disabledReason || action.label, "data-run-organization": action.id },
      });
  return `<div class="run-organization-actions" role="group" aria-label="${escapeHtml(`${run.agentName} Run`)}">
    ${renderAction(pin)}
    ${archive ? renderAction(archive) : ""}
  </div>`;
}

export function runPersistenceBanner(copy: ShellCopy, snap: Snapshot): string {
  if (!snap.capabilities.runOrganization) return "";
  if (snap.runPersistenceRecovery) {
    return `<section class="run-persistence-banner" role="alert" data-run-persistence="recovery">
      <div><b>${escapeHtml(copy.runPersistenceRecoveryTitle)}</b><p>${escapeHtml(copy.runPersistenceRecoveryBody)}</p><small>${escapeHtml(snap.runPersistenceRecovery.detail)}</small></div>
      ${button({ id: "retry-run-persistence", label: copy.runPersistenceRecoveryRetry, busy: ui.runOrganizationPending.has("persistence:retry") }, { variant: "secondary" })}
    </section>`;
  }
  if (snap.runPersistenceWriteError || ui.runOrganizationRetry) {
    return `<section class="run-persistence-banner" role="alert" data-run-persistence="write-error">
      <div><b>${escapeHtml(copy.runPersistenceWriteFailed)}</b><small>${escapeHtml(snap.runPersistenceWriteError?.detail ?? ui.runOrganizationRetry?.message ?? "")}</small></div>
      ${ui.runOrganizationRetry ? button({ id: "retry-run-organization", label: runOrganizationLabels().persistenceRetryAction }, { variant: "secondary" }) : ""}
    </section>`;
  }
  return "";
}

function projectLabel(snap: Snapshot, projectId: string): string {
  return snap.projects.find((project) => project.id === projectId)?.name
    ?? `${runOrganizationLabels().removedProject} · ${projectId}`;
}

function archiveTelemetry(run: RunSummary): string {
  const labels = runOrganizationLabels();
  const lanes = run.telemetry ?? [];
  if (!lanes.length) return `<p class="muted">${escapeHtml(labels.telemetryEmpty)}</p>`;
  return `<div class="archive-telemetry">${lanes.map((lane) => `<div>
    <span><b>${escapeHtml(lane.model)}</b><small>${escapeHtml(lane.lane)}</small></span>
    <span><small>${escapeHtml(labels.totalTokens)}</small><b>${lane.tokens.total ?? "—"}</b></span>
    <span><small>TTFT</small><b>${lane.ttftMs == null ? "—" : `${lane.ttftMs} ms`}</b></span>
    <span><small>tok/s</small><b>${lane.tokensPerSec ?? "—"}</b></span>
  </div>`).join("")}</div>`;
}

function archivedAt(run: RunSummary): string {
  if (!run.archivedAtMs) return "—";
  return new Intl.DateTimeFormat(effectiveClientLanguage(), {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(new Date(run.archivedAtMs));
}

export function runArchivePage(copy: ShellCopy, snap: Snapshot): string {
  const labels = runOrganizationLabels();
  const archivedRuns = ui.archivedRunsHostId === snap.focusedHostId ? ui.archivedRuns : [];
  const projectIds = [...new Set(archivedRuns.map((run) => run.projectId))];
  const options: SelectOption[] = [
    { value: "", label: labels.allProjects, selected: !ui.archiveProjectFilter },
    ...projectIds.map((projectId) => ({
      value: projectId,
      label: projectLabel(snap, projectId),
      selected: ui.archiveProjectFilter === projectId,
    })),
  ];
  const visible = archivedRuns.filter((run) => !ui.archiveProjectFilter || run.projectId === ui.archiveProjectFilter);
  if (ui.archiveSelectedRunId && !visible.some((run) => run.id === ui.archiveSelectedRunId)) {
    ui.archiveSelectedRunId = "";
  }
  const selected = visible.find((run) => run.id === ui.archiveSelectedRunId) ?? visible[0];
  if (selected && !ui.archiveSelectedRunId) ui.archiveSelectedRunId = selected.id;
  const recentlyRestored = (snap.runs ?? []).find((run) => run.id === ui.recentlyRestoredRunId);
  const recovery = snap.runPersistenceRecovery
    ? notice({ status: "danger", role: "alert", message: `${copy.runPersistenceRecoveryTitle} ${copy.runPersistenceRecoveryBody} ${snap.runPersistenceRecovery.detail}` })
    : "";
  const list = ui.archivedRunsLoading
    ? `<p class="archive-empty" role="status">${escapeHtml(labels.archiveLoading)}</p>`
    : ui.archivedRunsError
      ? `${notice({ status: "danger", role: "alert", message: ui.archivedRunsError })}${button({ id: "reload-run-archive", label: labels.retry })}`
      : recovery
        ? recovery
        : !visible.length
          ? `<p class="archive-empty">${escapeHtml(ui.archiveProjectFilter ? labels.archiveFilteredEmpty : labels.archiveEmpty)}</p>`
          : visible.map((run) => {
              const restoreAllowed = runOrganizationWritable(snap)
                && (snap.projects.some((project) => project.id === run.projectId) || Boolean(snap.capabilities.projectRestore));
              const restorePending = ui.runOrganizationPending.has(actionKey("restore", run.id));
              return `<article class="archive-run-row ${selected?.id === run.id ? "selected" : ""}" data-archived-run="${escapeHtml(run.id)}">
                <button type="button" class="archive-run-main" data-act="select-archived-run" data-id="${escapeHtml(run.id)}">
                  <span>${escapeHtml(projectLabel(snap, run.projectId))}</span>
                  <b>${escapeHtml(run.unbound || !run.issueId ? copy.unboundIssue : run.issueId)}</b>
                  <small>${escapeHtml(run.agentName)} · ${escapeHtml(labels.archivedAt)} ${escapeHtml(archivedAt(run))}</small>
                </button>
                ${button({ id: "restore-run", label: labels.restore, disabled: !restoreAllowed, busy: restorePending, data: { id: run.id } }, { variant: "secondary", className: "archive-restore" })}
              </article>`;
            }).join("");
  const detail = selected
    ? `<section class="archive-run-detail" data-archive-detail="${escapeHtml(selected.id)}">
        <header><div><span>${escapeHtml(projectLabel(snap, selected.projectId))}</span><h2>${escapeHtml(selected.agentName)} · ${escapeHtml(selected.unbound || !selected.issueId ? copy.unboundIssue : selected.issueId)}</h2><small>${escapeHtml(labels.archivedAt)} ${escapeHtml(archivedAt(selected))}</small></div></header>
        <section><h3>${escapeHtml(labels.telemetry)}</h3>${archiveTelemetry(selected)}</section>
        <section class="archive-output"><h3>${escapeHtml(labels.recentOutput)}</h3><pre aria-readonly="true">${escapeHtml(selected.recentOutput ?? "")}</pre></section>
      </section>`
    : `<section class="archive-run-detail archive-detail-empty"><p>${escapeHtml(labels.selectRun)}</p></section>`;
  return `<section class="run-archive-page" data-primary-page="run-archive">
    <div class="content-toolbar" data-page-toolbar>
      <div class="board-head"><div class="board-head-row"><div><h1>${escapeHtml(labels.archive)}</h1><p>${escapeHtml(labels.archiveHint)}</p></div></div></div>
    </div>
    ${recentlyRestored ? `<div class="archive-restored" role="status"><span>${escapeHtml(labels.restored)}</span>${button({ id: "open-restored-run", label: labels.openRun, data: { id: recentlyRestored.id } }, { variant: "primary" })}</div>` : ""}
    <div class="archive-controls"><label>${escapeHtml(labels.projectFilter)}${selectControl({ options, attributes: { "data-archive-filter": "project" } })}</label></div>
    <div class="run-archive-layout"><div class="archive-run-list">${list}</div>${detail}</div>
  </section>`;
}

function conflictMessage(result: Extract<RunRestoreResult, { status: "conflict" }>): string {
  const labels = runOrganizationLabels();
  if (result.reason === "tombstone-missing") return labels.conflictTombstoneMissing;
  if (result.reason === "directory-missing") return labels.conflictDirectoryMissing;
  if (result.reason === "directory-in-use") {
    return `${labels.conflictDirectoryInUse}${result.conflictingProjectId ? ` (${result.conflictingProjectId})` : ""}`;
  }
  if (result.reason === "tombstone-revision-changed") return labels.conflictRevisionChanged;
  return labels.conflictActiveProject;
}

export function restoreRunDialog(copy: ShellCopy): string {
  const state = ui.restoreRunDialog;
  if (!state) return "";
  const labels = runOrganizationLabels();
  if (state.result.status === "conflict") {
    const tombstone = state.result.tombstone;
    const details = tombstone ? `<dl class="restore-project-details">
      <div><dt>${escapeHtml(labels.originalProjectId)}</dt><dd>${escapeHtml(tombstone.projectId)}</dd></div>
      <div><dt>${escapeHtml(labels.originalProjectName)}</dt><dd>${escapeHtml(tombstone.name)}</dd></div>
      <div><dt>${escapeHtml(labels.originalDirectory)}</dt><dd>${escapeHtml(tombstone.localPath)}</dd></div>
    </dl>` : "";
    return confirmationDialog({
      id: "restore-run",
      title: labels.conflictTitle,
      body: `${notice({ status: "danger", role: "alert", message: conflictMessage(state.result) })}${details}`,
      closeLabel: labels.close,
      cancelLabel: labels.close,
      error: state.error,
    });
  }
  const tombstone = state.result.tombstone;
  const body = `${notice({ status: "danger", message: labels.restoreProjectBody })}<dl class="restore-project-details">
    <div><dt>${escapeHtml(labels.originalProjectId)}</dt><dd>${escapeHtml(tombstone.projectId)}</dd></div>
    <div><dt>${escapeHtml(labels.originalProjectName)}</dt><dd>${escapeHtml(tombstone.name)}</dd></div>
    <div><dt>${escapeHtml(labels.originalDirectory)}</dt><dd>${escapeHtml(tombstone.localPath)}</dd></div>
  </dl>`;
  return confirmationDialog({
    id: "restore-run",
    title: labels.restoreProjectTitle,
    body,
    closeLabel: labels.close,
    cancelLabel: copy.cancel,
    confirm: { id: "confirm-restore-run", label: labels.restoreProjectConfirm, destructive: true, busy: state.pending },
    busy: state.pending,
    error: state.error,
  });
}
