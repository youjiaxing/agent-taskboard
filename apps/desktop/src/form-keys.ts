import { RpcHttpError, type BoardSnapshot, type FormKey, type IssueContentDraft, type IssueDetail, type IssueLink, type IssueRelationDraft, type IssueSearchDraft } from "./protocol";
import { notice } from "./components/primitives";
import { render } from "./render/app";
import { ui } from "./ui";

export function issueDraftKey(issueId: string): string {
  return JSON.stringify([ui.snapshot?.focusedHostId, ui.snapshot?.focusedProjectId, issueId]);
}

export function issueCreateFormKey(projectId: string): FormKey {
  return `issue-create:${projectId}`;
}

export function issueEditFormKey(issueId: string): FormKey {
  return `issue-edit:${issueDraftKey(issueId)}`;
}

export function issueCommentFormKey(issueId: string): FormKey {
  return `issue-comment:${issueDraftKey(issueId)}`;
}

export function issueParentFormKey(issueId: string): FormKey {
  return `issue-parent:${issueDraftKey(issueId)}`;
}

export function issueBlockersFormKey(issueId: string): FormKey {
  return `issue-blockers:${issueDraftKey(issueId)}`;
}

export function issueOpenFormKey(issueId: string): FormKey {
  return `issue-open:${issueDraftKey(issueId)}`;
}

export function issueSearchFormKey(projectId: string): FormKey {
  return `issue-search:${projectId}`;
}

export function editableIssueSearchDraft(projectId: string): IssueSearchDraft {
  if (ui.issueSearchDraft?.projectId === projectId) return ui.issueSearchDraft;
  const search = ui.snapshot?.board?.search;
  ui.issueSearchDraft = {
    projectId,
    title: search?.title ?? "",
    triageRole: search?.triageRole ?? "",
    state: search?.state ?? "all",
  };
  return ui.issueSearchDraft;
}

export function injectFormKey(runId: string): FormKey {
  return `inject-run:${runId}`;
}

export function changeNoteFormKey(runId: string): FormKey {
  return `change-note:${runId}`;
}

export function launchFormKey(projectId: string): FormKey {
  return `launch:${projectId}`;
}

export function usageCustomFormKey(hostId: string): FormKey {
  return `usage-custom:${hostId}`;
}

export function revokeClientFormKey(clientId: string): FormKey {
  return `revoke-client:${clientId}`;
}

export function issueDocumentBody(issue: IssueDetail): string {
  const document = issue.document;
  return document.kind === "ready" || document.kind === "stale"
    ? document.body ?? ""
    : "";
}

export function editableIssueBody(issue: IssueDetail): string {
  const raw = issueDocumentBody(issue);
  return issue.document.kind === "ready" ? issue.document.editableBody ?? raw : raw;
}

export function editableIssueDraft(issue: IssueDetail): IssueContentDraft {
  const existing = ui.issueEditDrafts.get(issueDraftKey(issue.id));
  if (existing) return existing;
  const title = issue.title;
  const body = editableIssueBody(issue);
  const draft = { title, body, baseTitle: title, baseBody: body };
  ui.issueEditDrafts.set(issueDraftKey(issue.id), draft);
  return draft;
}

export function editableIssueRelations(issue: IssueDetail): IssueRelationDraft {
  const existing = ui.issueRelationDrafts.get(issueDraftKey(issue.id));
  if (existing) return existing;
  const parent = issue.parent?.id ?? "";
  const blockedBy = issue.blockedBy.map((link) => link.id);
  const draft = {
    parent,
    blockedBy,
    baseParent: parent,
    baseBlockedBy: [...blockedBy],
  };
  ui.issueRelationDrafts.set(issueDraftKey(issue.id), draft);
  return draft;
}

export function issueOptionLabel(link: IssueLink): string {
  const number = link.number == null ? "" : `#${link.number} `;
  return `${number}${link.title || link.id}`.trim();
}

export function issueOptionList(board: BoardSnapshot, issue: IssueDetail): IssueLink[] {
  const options = [...(board.issueOptions ?? [])];
  const known = new Set(options.map((option) => option.id));
  for (const relation of [issue.parent, ...issue.blockedBy]) {
    if (relation && !known.has(relation.id)) {
      options.push(relation);
      known.add(relation.id);
    }
  }
  return options.filter((option) => option.id !== issue.id);
}

export function formFeedback(key: FormKey): string {
  const error = ui.formOperations.errors.get(key);
  return error ? notice({ status: "danger", role: "alert", className: "form-feedback", message: error }) : "";
}

export function clearFormOperation(key: FormKey): void {
  ui.formOperations.errors.delete(key);
  ui.formOperations.conflicts.delete(key);
}

export async function runFormOperation(key: FormKey, operation: () => Promise<void>): Promise<boolean> {
  if (ui.formOperations.pending.has(key)) return false;
  const originScope = issueDraftKey("");
  const active = document.activeElement;
  const activeField = active instanceof HTMLInputElement || active instanceof HTMLTextAreaElement
    ? {
        id: active.id,
        selectionStart: active.selectionStart,
        selectionEnd: active.selectionEnd,
      }
    : active instanceof HTMLSelectElement
      ? { id: active.id, selectionStart: null, selectionEnd: null }
      : null;
  let succeeded = false;
  ui.formOperations.pending.add(key);
  ui.formOperations.errors.delete(key);
  ui.formOperations.conflicts.delete(key);
  render();
  try {
    await operation();
    succeeded = true;
    return true;
  } catch (error) {
    if (error instanceof RpcHttpError && error.conflict) {
      ui.formOperations.conflicts.set(key, error.conflict);
    } else {
      ui.formOperations.errors.set(key, error instanceof Error ? error.message : String(error));
    }
    return false;
  } finally {
    ui.formOperations.pending.delete(key);
    render();
    if (!succeeded && activeField?.id && originScope === issueDraftKey("")) {
      const detailScroll = ui.app.querySelector<HTMLElement>(".detail-scroll");
      const detailScrollPosition = detailScroll
        ? { top: detailScroll.scrollTop, left: detailScroll.scrollLeft }
        : null;
      const field = document.getElementById(activeField.id);
      if (field instanceof HTMLInputElement || field instanceof HTMLTextAreaElement) {
        field.focus();
        if (activeField.selectionStart != null && activeField.selectionEnd != null) {
          field.setSelectionRange(activeField.selectionStart, activeField.selectionEnd);
        }
      } else if (field instanceof HTMLSelectElement) {
        field.focus();
      }
      if (detailScroll && detailScrollPosition) {
        detailScroll.scrollTop = detailScrollPosition.top;
        detailScroll.scrollLeft = detailScrollPosition.left;
      }
    }
  }
}
