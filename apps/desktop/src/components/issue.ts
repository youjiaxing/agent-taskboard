import { escapeHtml } from "../client-utils";
import type { ShellCopy } from "../protocol";
import { actionAttributes, badge, button, htmlAttributes, type ActionDescriptor } from "./primitives";

/** Board column an Issue belongs to. The page resolves it from the Host board projection. */
export type IssueLaneState = "blocked" | "frontier" | "inProgress" | "recentlyCompleted";

export type IssueActivityState = "running" | "waiting" | "execution-stopped";

export type IssueIdentityView = {
  number: number;
  title: string;
};

export type IssueDisplayTag = {
  label: string;
  tone?: "info" | "success" | "warning" | "danger" | "neutral";
};

export function issueIdentity(issue: IssueIdentityView): string {
  return `<div class="issue-id">#${issue.number}</div>
    <div class="issue-title">${escapeHtml(issue.title)}</div>`;
}

export function issueTags(tags: IssueDisplayTag[]): string {
  return tags
    .map((tag) => badge({ label: tag.label, status: tag.tone, className: "tag" }))
    .join("");
}

const LANE_STATE_LABEL: Record<IssueLaneState, (copy: ShellCopy) => string> = {
  blocked: (copy) => copy.colBlocked,
  frontier: (copy) => copy.colFrontier,
  inProgress: (copy) => copy.colInProgress,
  recentlyCompleted: (copy) => copy.colRecent,
};

const LANE_STATE_TONE: Record<IssueLaneState, IssueDisplayTag["tone"]> = {
  blocked: "warning",
  frontier: "info",
  inProgress: "success",
  recentlyCompleted: "neutral",
};

/** The caller has already resolved the column; this only maps it onto a Badge. */
export function issueStateBadge(copy: ShellCopy, state: IssueLaneState): string {
  return badge({
    label: LANE_STATE_LABEL[state](copy),
    status: LANE_STATE_TONE[state],
    className: "tag issue-state",
  });
}

export type IssueCardAction = {
  action: ActionDescriptor;
  variant?: "primary" | "secondary" | "ghost" | "danger";
};

export type IssueCardView = {
  id: string;
  identity: IssueIdentityView;
  state: IssueLaneState;
  activity?: IssueActivityState | null;
  selected?: boolean;
  tags: IssueDisplayTag[];
  recentAction?: string;
  recentOutput?: string;
  /** Workspace entry owned by the calling page. */
  entry: ActionDescriptor;
  actions: IssueCardAction[];
};

export function issueCard(card: IssueCardView): string {
  const className = [
    "issue-card",
    card.selected ? "sel" : "",
    card.activity ?? "",
    card.state === "recentlyCompleted" ? "recently-completed subdued" : "",
  ]
    .filter(Boolean)
    .join(" ");
  const tags = issueTags(card.tags);
  const actions = card.actions
    .map((item) => button(item.action, { variant: item.variant }))
    .join("");
  return `<article class="${className}" data-issue-id="${escapeHtml(card.id)}">
    <button${htmlAttributes({
      type: "button",
      class: "issue-card-main",
      "data-issue-id": card.id,
      ...actionAttributes(card.entry),
    })}>
      ${issueIdentity(card.identity)}
      ${tags ? `<div class="issue-tags">${tags}</div>` : ""}
      ${card.recentAction ? `<span class="issue-run-action">${escapeHtml(card.recentAction)}</span>` : ""}
    </button>
    ${card.recentOutput ? `<pre class="issue-run-preview" aria-readonly="true">${escapeHtml(card.recentOutput)}</pre>` : ""}
    ${actions ? `<div class="issue-card-actions">${actions}</div>` : ""}
  </article>`;
}
