import type { BoardSnapshot, FormKey, IssueContentDraft, IssueDetail, IssueLink, IssueRelationDraft, IssueSearchDraft } from "./protocol";
import { escapeHtml } from "./client-utils";
import { render } from "./render/app";
import { ui } from "./ui";

export function issueCreateFormKey(projectId: string): FormKey {
  return `issue-create:${projectId}`;
}

export function issueEditFormKey(issueId: string): FormKey {
  return `issue-edit:${issueId}`;
}

export function issueCommentFormKey(issueId: string): FormKey {
  return `issue-comment:${issueId}`;
}

export function issueParentFormKey(issueId: string): FormKey {
  return `issue-parent:${issueId}`;
}

export function issueBlockersFormKey(issueId: string): FormKey {
  return `issue-blockers:${issueId}`;
}

export function issueOpenFormKey(issueId: string): FormKey {
  return `issue-open:${issueId}`;
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

export function issueDocumentBody(issue: IssueDetail): string {
  const document = issue.document;
  return document.kind === "ready" || document.kind === "stale"
    ? document.body ?? ""
    : "";
}

export function editableIssueBody(issue: IssueDetail): string {
  const raw = issueDocumentBody(issue);
  const project = ui.snapshot?.projects.find((item) => item.id === ui.snapshot?.board?.projectId);
  if (project?.tracker !== "local-markdown") return raw;
  const lines = raw.replace(/\r\n?/g, "\n").split("\n");
  let start = 0;
  while (start < lines.length && !/^\s*#\s+/.test(lines[start])) start += 1;
  if (start < lines.length) start += 1;
  const metadata = /^\s*\**(?:status|type|assignees?|part of|parent|blocked by|closed)\s*:/i;
  while (start < lines.length && (lines[start].trim() === "" || metadata.test(lines[start]))) start += 1;
  const body = lines.slice(start);
  const comments = body.findIndex((line) => /^\s*##\s+comments\s*$/i.test(line));
  return (comments >= 0 ? body.slice(0, comments) : body).join("\n").trim();
}

export function editableIssueDraft(issue: IssueDetail): IssueContentDraft {
  const existing = ui.issueEditDrafts.get(issue.id);
  if (existing) return existing;
  const draft = { title: issue.title, body: editableIssueBody(issue) };
  ui.issueEditDrafts.set(issue.id, draft);
  return draft;
}

export function editableIssueRelations(issue: IssueDetail): IssueRelationDraft {
  const existing = ui.issueRelationDrafts.get(issue.id);
  if (existing) return existing;
  const draft = {
    parent: issue.parent?.id ?? "",
    blockedBy: issue.blockedBy.map((link) => link.id),
  };
  ui.issueRelationDrafts.set(issue.id, draft);
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
  return error ? `<p class="notice bad form-feedback">${escapeHtml(error)}</p>` : "";
}

export function clearFormOperation(key: FormKey): void {
  ui.formOperations.errors.delete(key);
}

export async function runFormOperation(key: FormKey, operation: () => Promise<void>): Promise<boolean> {
  if (ui.formOperations.pending.has(key)) return false;
  ui.formOperations.pending.add(key);
  ui.formOperations.errors.delete(key);
  render();
  try {
    await operation();
    return true;
  } catch (error) {
    ui.formOperations.errors.set(key, error instanceof Error ? error.message : String(error));
    return false;
  } finally {
    ui.formOperations.pending.delete(key);
    render();
  }
}
