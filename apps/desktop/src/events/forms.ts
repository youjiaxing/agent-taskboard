import { applyLaunchDependentDefaults, applyLocalPath, expectedOpening, launchValuesForHost, refreshIntentChoices, refreshLaunchFieldOptions, refreshLaunchWarnings, requestDesktopNotificationPermission, scheduleLaunchPreview, setStartAtLogin, supersedeProjectInference } from "../launch-session";
import { issueDraftKey, changeNoteFormKey, editableIssueDraft, editableIssueRelations, editableIssueSearchDraft, injectFormKey, issueBlockersFormKey, issueCommentFormKey, issueCreateFormKey, issueEditFormKey, issueParentFormKey, issueSearchFormKey, launchFormKey, runFormOperation, usageCustomFormKey } from "../form-keys";
import { CUSTOM_VALUE, launchFieldOptions } from "../render/run";
import { loadSelectedIssueDocument, loadViewChanges, rpc, rpcDetached } from "../rpc";
import { render } from "../render/app";
import { toLocalInput } from "../client-utils";
import { ui } from "../ui";

export function bindFormEvents(): void {
ui.app.addEventListener("submit", async (event) => {
  const create = (event.target as HTMLElement | null)?.closest<HTMLFormElement>("form[data-act='issue-create']");
  if (create && ui.snapshot) {
    event.preventDefault();
    const projectId = ui.createIssueProjectId || ui.snapshot.focusedProjectId;
    if (!projectId) return;
    const data = new FormData(create);
    const draft = {
      title: String(data.get("title") ?? ""),
      body: String(data.get("body") ?? ""),
    };
    ui.createIssueDraft = draft;
    if (!draft.title.trim()) return;
    const success = await runFormOperation(issueCreateFormKey(projectId), async () => {
      await rpcDetached("createIssue", {
        projectId,
        title: draft.title,
        body: draft.body,
      });
    });
    if (success) {
      ui.createIssueOpen = false;
      ui.createIssueProjectId = "";
      ui.createIssueDraft = { title: "", body: "" };
      render();
    }
    return;
  }
  const edit = (event.target as HTMLElement | null)?.closest<HTMLFormElement>("form[data-act='issue-edit']");
  if (edit && ui.snapshot) {
    event.preventDefault();
    const issueId = edit.dataset.id;
    if (!issueId) return;
    const draftKey = issueDraftKey(issueId);
    const issue = ui.snapshot.board?.selected?.id === issueId ? ui.snapshot.board.selected : null;
    if (!issue) return;
    const storedDraft = ui.issueEditDrafts.get(draftKey);
    if (issue.document.kind !== "ready" && !storedDraft) return;
    const data = new FormData(edit);
    const existing = storedDraft ?? editableIssueDraft(issue);
    const draft = {
      ...existing,
      title: String(data.get("title") ?? ""),
      body: String(data.get("body") ?? ""),
    };
    ui.issueEditDrafts.set(draftKey, draft);
    if (!draft.title.trim()) return;
    const overwriteConflict = (event as SubmitEvent).submitter instanceof HTMLButtonElement
      && (event as SubmitEvent).submitter?.dataset.conflictPolicy === "overwrite";
    const success = await runFormOperation(issueEditFormKey(issueId), async () => {
      await rpcDetached("updateIssue", {
        issueId,
        title: draft.title,
        body: draft.body,
        base: { title: draft.baseTitle, body: draft.baseBody },
        conflictPolicy: overwriteConflict ? "overwrite" : undefined,
      });
      await loadSelectedIssueDocument(true);
    });
    if (success) {
      ui.issueEditDrafts.delete(draftKey);
      ui.issueEditOpenIds.delete(draftKey);
      render();
    }
    return;
  }
  const comment = (event.target as HTMLElement | null)?.closest<HTMLFormElement>("form[data-act='issue-comment']");
  if (comment && ui.snapshot) {
    event.preventDefault();
    const issueId = comment.dataset.id;
    if (!issueId) return;
    const draftKey = issueDraftKey(issueId);
    const body = String(new FormData(comment).get("body") ?? "");
    ui.issueCommentDrafts.set(draftKey, body);
    if (!body.trim()) return;
    const success = await runFormOperation(issueCommentFormKey(issueId), async () => {
      await rpcDetached("addIssueComment", { issueId, body });
      await loadSelectedIssueDocument(true);
    });
    if (success) {
      ui.issueCommentDrafts.delete(draftKey);
      render();
    }
    return;
  }
  const parentForm = (event.target as HTMLElement | null)?.closest<HTMLFormElement>("form[data-act='issue-parent']");
  if (parentForm && ui.snapshot) {
    event.preventDefault();
    const issueId = parentForm.dataset.id;
    if (!issueId) return;
    const draftKey = issueDraftKey(issueId);
    const parent = String(new FormData(parentForm).get("parent") ?? "");
    const issue = ui.snapshot.board?.selected?.id === issueId ? ui.snapshot.board.selected : null;
    if (!issue) return;
    const current = editableIssueRelations(issue);
    ui.issueRelationDrafts.set(draftKey, { ...current, parent });
    const overwriteConflict = (event as SubmitEvent).submitter instanceof HTMLButtonElement
      && (event as SubmitEvent).submitter?.dataset.conflictPolicy === "overwrite";
    const success = await runFormOperation(issueParentFormKey(issueId), async () => {
      await rpcDetached("setIssueParent", {
        issueId,
        parent,
        base: { parent: current.baseParent || null },
        conflictPolicy: overwriteConflict ? "overwrite" : undefined,
      });
    });
    if (success) {
      ui.issueRelationDrafts.delete(draftKey);
      render();
    }
    return;
  }
  const blockersForm = (event.target as HTMLElement | null)?.closest<HTMLFormElement>("form[data-act='issue-blockers']");
  if (blockersForm && ui.snapshot) {
    event.preventDefault();
    const issueId = blockersForm.dataset.id;
    if (!issueId) return;
    const draftKey = issueDraftKey(issueId);
    const data = new FormData(blockersForm);
    const blockedBy = data.getAll("blockedBy").map((value) => String(value));
    const issue = ui.snapshot.board?.selected?.id === issueId ? ui.snapshot.board.selected : null;
    if (!issue) return;
    const current = editableIssueRelations(issue);
    ui.issueRelationDrafts.set(draftKey, { ...current, blockedBy });
    const overwriteConflict = (event as SubmitEvent).submitter instanceof HTMLButtonElement
      && (event as SubmitEvent).submitter?.dataset.conflictPolicy === "overwrite";
    const success = await runFormOperation(issueBlockersFormKey(issueId), async () => {
      await rpcDetached("setIssueBlockedBy", {
        issueId,
        blockedBy,
        base: { blockedBy: current.baseBlockedBy },
        conflictPolicy: overwriteConflict ? "overwrite" : undefined,
      });
    });
    if (success) {
      ui.issueRelationDrafts.delete(draftKey);
      render();
    }
    return;
  }
  const search = (event.target as HTMLElement | null)?.closest<HTMLFormElement>("form[data-act='issue-search']");
  if (search && ui.snapshot) {
    event.preventDefault();
    const data = new FormData(search);
    const draft = {
      projectId: ui.snapshot.focusedProjectId,
      title: String(data.get("title") ?? ""),
      triageRole: String(data.get("triageRole") ?? ""),
      state: String(data.get("state") ?? "all"),
    };
    ui.issueSearchDraft = draft;
    const projectId = ui.snapshot.focusedProjectId;
    const key = issueSearchFormKey(projectId);
    const success = await runFormOperation(key, async () => {
      await rpc("searchIssues", {
        projectId,
        title: draft.title,
        triageRole: draft.triageRole,
        state: draft.state,
      });
    });
    if (success) {
      ui.issueSearchDraft = null;
      ui.keyboardCursorIssueId = "";
      render();
    }
    return;
  }
  const inject = (event.target as HTMLElement | null)?.closest<HTMLFormElement>("form[data-act='inject-run']");
  if (inject && ui.snapshot) {
    event.preventDefault();
    const runId = inject.dataset.id;
    const input = inject.querySelector<HTMLInputElement>("input[name='text']");
    const text = input?.value ?? "";
    if (!runId || !text.trim()) return;
    ui.terminalInputDrafts.set(runId, text);
    const success = await runFormOperation(injectFormKey(runId), async () => {
      await rpc("injectRunInput", { runId, text });
    });
    if (success) {
      ui.terminalInputDrafts.delete(runId);
      render();
    }
    return;
  }
  const noteForm = (event.target as HTMLElement | null)?.closest<HTMLFormElement>("form[data-act='write-note']");
  if (!noteForm || !ui.snapshot || !ui.noteTarget || !ui.changesView) return;
  event.preventDefault();
  const input = noteForm.querySelector<HTMLInputElement>("input[name='text']");
  const text = input?.value ?? ui.noteDraft;
  if (!text.trim()) return;
  ui.noteDraft = text;
  const target = { ...ui.noteTarget };
  const runId = ui.changesView.runId;
  const success = await runFormOperation(changeNoteFormKey(runId), async () => {
    await rpc("writeChangeNote", {
      runId,
      repo: target.repo,
      path: target.path,
      line: target.line,
      text,
    });
    await loadViewChanges(runId, ui.changesScope);
  });
  if (success) {
    ui.noteDraft = "";
    ui.noteTarget = null;
    render();
  }
});

ui.app.addEventListener("input", (event) => {
  const target = event.target as HTMLInputElement | null;
  if (!target) return;
  const createForm = target.closest<HTMLFormElement>("form[data-form='issue-create']");
  if (createForm && (target.name === "title" || target.name === "body")) {
    ui.createIssueDraft = { ...ui.createIssueDraft, [target.name]: target.value };
    return;
  }
  const editForm = target.closest<HTMLFormElement>("form[data-form='issue-edit']");
  if (editForm && editForm.dataset.id && (target.name === "title" || target.name === "body")) {
    const current = ui.issueEditDrafts.get(issueDraftKey(editForm.dataset.id)) ?? {
      title: "",
      body: "",
      baseTitle: "",
      baseBody: "",
    };
    ui.issueEditDrafts.set(issueDraftKey(editForm.dataset.id), { ...current, [target.name]: target.value });
    return;
  }
  const commentForm = target.closest<HTMLFormElement>("form[data-form='issue-comment']");
  if (commentForm?.dataset.id && target.name === "body") {
    ui.issueCommentDrafts.set(issueDraftKey(commentForm.dataset.id), target.value);
    return;
  }
  const injectForm = target.closest<HTMLFormElement>("form[data-act='inject-run']");
  if (injectForm?.dataset.id && target.name === "text") {
    ui.terminalInputDrafts.set(injectForm.dataset.id, target.value);
    return;
  }
  const usageForm = target.closest<HTMLFormElement>("form[data-act='usage-custom']");
  if (usageForm && (target.name === "from" || target.name === "to")) {
    const usage = ui.snapshot?.usage;
    if (usage && ui.usageCustomDraft?.hostId !== ui.snapshot?.focusedHostId) {
      ui.usageCustomDraft = {
        hostId: ui.snapshot?.focusedHostId ?? "",
        from: toLocalInput(usage.fromMs),
        to: toLocalInput(usage.toMs),
      };
    }
    if (ui.usageCustomDraft) {
      ui.usageCustomDraft[target.name] = target.value;
    }
    return;
  }
  const searchForm = target.closest<HTMLFormElement>("form[data-act='issue-search']");
  if (searchForm && target.name === "title") {
    editableIssueSearchDraft(ui.snapshot?.focusedProjectId ?? "").title = target.value;
    return;
  }
  if (target.getAttribute("data-field") === "graphSearch") {
    ui.graphListQuery = target.value;
    ui.graphListLimit = 50;
    render();
    return;
  }
  if (!ui.formOpen) return;
  const field = target.getAttribute("data-field");
  if (field === "name" || field === "localPath" || field === "githubHost" || field === "repository") {
    ui.formDraft = { ...ui.formDraft, [field]: target.value };
    if (field === "name") ui.autoFilledProjectName = "";
    if (field === "localPath") target.title = target.value;
  }
});

ui.app.addEventListener("toggle", (event) => {
  const details = event.target;
  if (!(details instanceof HTMLDetailsElement)) return;
  if (details.dataset.section !== "issue-maintenance" || !details.dataset.id) return;
  if (details.open) ui.issueMaintenanceOpen.add(issueDraftKey(details.dataset.id));
  else ui.issueMaintenanceOpen.delete(issueDraftKey(details.dataset.id));
}, true);

ui.app.addEventListener("change", async (event) => {
  const target = event.target as HTMLElement | null;
  if (!target || !ui.snapshot) return;
  const issueParent = target.closest<HTMLFormElement>("form[data-form='issue-parent']");
  if (issueParent?.dataset.id && target instanceof HTMLSelectElement && target.name === "parent") {
    const issue = ui.snapshot.board?.selected?.id === issueParent.dataset.id ? ui.snapshot.board.selected : null;
    if (issue) {
      const current = editableIssueRelations(issue);
      ui.issueRelationDrafts.set(issueDraftKey(issue.id), { ...current, parent: target.value });
    }
    return;
  }
  const issueBlockers = target.closest<HTMLFormElement>("form[data-form='issue-blockers']");
  if (issueBlockers?.dataset.id && target instanceof HTMLSelectElement && target.name === "blockedBy") {
    const issue = ui.snapshot.board?.selected?.id === issueBlockers.dataset.id ? ui.snapshot.board.selected : null;
    if (issue) {
      const current = editableIssueRelations(issue);
      ui.issueRelationDrafts.set(issueDraftKey(issue.id), { ...current, blockedBy: [...target.selectedOptions].map((option) => option.value) });
    }
    return;
  }
  if (target.getAttribute("data-field") === "localPath" && "value" in target) {
    applyLocalPath((target as HTMLInputElement).value, true);
    return;
  }
  if (target.getAttribute("data-field") === "startAtLogin" && "checked" in target) {
    await setStartAtLogin((target as HTMLInputElement).checked);
    render();
  }
  if (target.getAttribute("data-field") === "refreshInterval" && "value" in target) {
    const seconds = Number((target as HTMLInputElement).value);
    if (!Number.isFinite(seconds)) return;
    await rpc("setRefreshInterval", { intervalMs: Math.max(0, seconds) * 1000 });
    render();
  }
  if (target.getAttribute("data-field") === "recentLimit" && "value" in target) {
    const limit = Number((target as HTMLInputElement).value);
    if (!Number.isFinite(limit)) return;
    await rpc("setRecentCompletedLimit", { limit });
    render();
  }
  if (target.getAttribute("data-field") === "showEndedRuns" && "checked" in target) {
    ui.overviewShowEnded = (target as HTMLInputElement).checked;
    render();
  }
  if (target.getAttribute("data-field") === "closedContext" && "checked" in target) {
    await rpc("setShowClosedGraphContext", {
      show: (target as HTMLInputElement).checked,
    });
    render();
  }
  if (target.getAttribute("data-field") === "commandPreview" && "checked" in target) {
    await rpc("setShowCommandPreview", {
      show: (target as HTMLInputElement).checked,
    });
    render();
  }
  if (
    (target.getAttribute("data-field") === "notifyDesktop" ||
      target.getAttribute("data-field") === "notifySound") &&
    "checked" in target
  ) {
    const desktop =
      target.getAttribute("data-field") === "notifyDesktop"
        ? (target as HTMLInputElement).checked
        : Boolean(ui.snapshot.notifyDesktop);
    const sound =
      target.getAttribute("data-field") === "notifySound"
        ? (target as HTMLInputElement).checked
        : Boolean(ui.snapshot.notifySound);
    if (desktop) await requestDesktopNotificationPermission();
    await rpc("setNotificationPrefs", { desktop, sound });
    render();
  }
  if (target.getAttribute("data-field") === "hostAutoAdvance" && "checked" in target) {
    await rpc("setHostAutoAdvance", {
      enabled: (target as HTMLInputElement).checked,
    });
    render();
  }
  if (target.getAttribute("data-field") === "projectAutoAdvance" && "checked" in target) {
    const projectId = ui.snapshot.focusedProjectId;
    if (projectId) {
      await rpc("setProjectAutoAdvance", {
        projectId,
        enabled: (target as HTMLInputElement).checked,
      });
      render();
    }
  }
  if (target.getAttribute("data-field") === "restoreAutoAdvance" && "checked" in target) {
    const projectId = ui.snapshot.focusedProjectId;
    if (projectId) {
      await rpc("setProjectRestoreAutoAdvance", {
        projectId,
        enabled: (target as HTMLInputElement).checked,
      });
      render();
    }
  }
  if (target.getAttribute("data-field") === "restoreDelay" && "value" in target) {
    const projectId = ui.snapshot.focusedProjectId;
    const seconds = Number((target as HTMLInputElement).value);
    if (projectId && Number.isFinite(seconds)) {
      await rpc("setProjectRestoreDelay", {
        projectId,
        delayMs: Math.max(0, seconds) * 1000,
      });
      render();
    }
  }
  const launchId = target.getAttribute("data-launch");
  if (launchId && ui.launchDraft) {
    if (target instanceof HTMLInputElement && target.type === "checkbox") {
      ui.launchDraft.values[launchId] = target.checked ? "true" : "false";
    } else if ("value" in target) {
      const next = (target as HTMLInputElement | HTMLSelectElement).value;
      ui.launchDraft.values[launchId] = next;
      if (next !== CUSTOM_VALUE) applyLaunchDependentDefaults(launchId);
    }
    refreshLaunchWarnings();
    refreshLaunchFieldOptions();
    scheduleLaunchPreview();
  }
});

ui.app.addEventListener("input", (event) => {
  const target = event.target as HTMLElement | null;
  if (!target) return;
  const field = target.getAttribute("data-field");
  if (field === "address" && "value" in target) {
    ui.pairingAddress = (target as HTMLInputElement).value;
  }
  if (field === "paste" && "value" in target) {
    ui.pairingPaste = (target as HTMLTextAreaElement).value;
  }
  if (target.closest(".note-form") && "value" in target) {
    ui.noteDraft = (target as HTMLInputElement).value;
  }
  if (
    (field === "name" || field === "githubHost" || field === "repository") &&
    "value" in target
  ) {
    ui.formDraft = { ...ui.formDraft, [field]: (target as HTMLInputElement).value };
  }
  if (field === "openingText" && ui.launchDraft && "value" in target) {
    ui.launchDraft.openingText = (target as HTMLTextAreaElement).value;
    if (!ui.launchDraft.intentId) {
      ui.launchDraft.values["initial-instruction"] = ui.launchDraft.openingText;
      ui.launchDraft.custom = false;
    } else if (ui.snapshot?.launchForm) {
      ui.launchDraft.custom =
        ui.launchDraft.openingText.trim() !== expectedOpening(ui.snapshot.launchForm, ui.launchDraft).trim();
      refreshIntentChoices();
    }
  }
  const customLaunchId = target.getAttribute("data-launch-custom");
  if (customLaunchId && ui.launchDraft && "value" in target) {
    ui.launchDraft.values[customLaunchId] = (target as HTMLInputElement).value;
    applyLaunchDependentDefaults(customLaunchId);
    refreshLaunchWarnings();
    refreshLaunchFieldOptions();
    scheduleLaunchPreview();
  }
  const launchId = target.getAttribute("data-launch");
  if (
    launchId
    && ui.launchDraft
    && "value" in target
    && !(target instanceof HTMLInputElement && target.type === "checkbox")
  ) {
    const next = (target as HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement).value;
    ui.launchDraft.values[launchId] = next;
    if (next !== CUSTOM_VALUE) applyLaunchDependentDefaults(launchId);
    refreshLaunchWarnings();
    refreshLaunchFieldOptions();
    scheduleLaunchPreview();
  }
});

ui.app.addEventListener("change", async (event) => {
  const target = event.target as HTMLElement | null;
  const launchSelectId = target?.getAttribute("data-launch-select");
  if (launchSelectId && ui.launchDraft && target instanceof HTMLSelectElement) {
    if (target.value === CUSTOM_VALUE) {
      const field = ui.snapshot?.launchForm?.fields.find((candidate) => candidate.id === launchSelectId);
      const options = field ? launchFieldOptions(field, ui.launchDraft.values) : [];
      if (options.includes(ui.launchDraft.values[launchSelectId] ?? "")) {
        ui.launchDraft.values[launchSelectId] = "";
      }
      const custom = ui.app?.querySelector<HTMLInputElement>(
        `[data-launch-custom="${CSS.escape(launchSelectId)}"]`,
      );
      if (custom) {
        custom.hidden = false;
        custom.required = Boolean(ui.snapshot?.launchForm?.fields.find(
          (field) => field.id === launchSelectId,
        )?.required);
        custom.focus();
      }
    } else {
      ui.launchDraft.values[launchSelectId] = target.value;
      applyLaunchDependentDefaults(launchSelectId);
      refreshLaunchFieldOptions();
    }
    refreshLaunchWarnings();
    scheduleLaunchPreview();
    return;
  }
  if (target instanceof HTMLSelectElement && target.closest("form[data-act='issue-search']")) {
    const draft = editableIssueSearchDraft(ui.snapshot?.focusedProjectId ?? "");
    if (target.name === "triageRole") draft.triageRole = target.value;
    if (target.name === "state") draft.state = target.value;
    return;
  }
  if (target?.getAttribute("data-overview-filter") === "project" && target instanceof HTMLSelectElement) {
    ui.overviewProjectId = target.value;
    render();
    return;
  }
  const filter = target?.getAttribute("data-usage-filter");
  if (!filter || !ui.snapshot?.usage || !(target instanceof HTMLSelectElement)) return;
  const next = {
    projectId: ui.snapshot.usage.filter.projectId ?? "",
    agentId: ui.snapshot.usage.filter.agentId ?? "",
    model: ui.snapshot.usage.filter.model ?? "",
  };
  if (filter === "projectId") next.projectId = target.value;
  if (filter === "agentId") next.agentId = target.value;
  if (filter === "model") next.model = target.value;
  await rpc("setUsageFilter", next);
  render();
});

ui.app.addEventListener("submit", async (event) => {
  const custom = (event.target as HTMLElement | null)?.closest<HTMLFormElement>("[data-act='usage-custom']");
  if (custom) {
    event.preventDefault();
    const data = new FormData(custom);
    const draft = {
      hostId: ui.snapshot?.focusedHostId ?? "",
      from: String(data.get("from") ?? ""),
      to: String(data.get("to") ?? ""),
    };
    const from = Date.parse(draft.from);
    const to = Date.parse(draft.to);
    if (Number.isNaN(from) || Number.isNaN(to)) return;
    ui.usageCustomDraft = draft;
    const key = usageCustomFormKey(ui.snapshot?.focusedHostId ?? "");
    const success = await runFormOperation(key, async () => {
      await rpc("setUsageRange", { range: "custom", fromMs: from, toMs: to });
    });
    if (success) {
      ui.usageCustomDraft = null;
      render();
    }
    return;
  }
  const launch = (event.target as HTMLElement | null)?.closest<HTMLFormElement>("[data-form='launch']");
  if (launch && ui.snapshot && ui.launchDraft) {
    event.preventDefault();
    const draft = {
      projectId: ui.launchDraft.projectId,
      issueId: ui.launchDraft.issueId,
      agentId: ui.launchDraft.agentId,
      values: launchValuesForHost(ui.launchDraft),
      openingText: ui.launchDraft.openingText,
    };
    await runFormOperation(launchFormKey(draft.projectId), async () => {
      await rpc("startUnboundRun", draft);
      ui.terminalPanelVisible = true;
    });
    return;
  }
  const form = (event.target as HTMLElement | null)?.closest<HTMLFormElement>("[data-form='project']");
  if (!form || !ui.snapshot) return;
  event.preventDefault();
  if (ui.projectOperation) return;
  supersedeProjectInference();
  ui.formError = "";
  ui.projectOperation = "save";
  render();
  try {
    if (ui.formOpen === "edit") {
      await rpc("editProject", { projectId: ui.formProjectId, ...ui.formDraft });
    } else {
      await rpc("registerProject", ui.formDraft);
    }
    ui.formOpen = null;
  } catch (error) {
    ui.formError = error instanceof Error ? error.message : String(error);
  } finally {
    ui.projectOperation = null;
  }
  render();
});


}
