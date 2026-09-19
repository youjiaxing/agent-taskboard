import { escapeHtml } from "../client-utils";
import type { RefreshStatus } from "../protocol";
import { button, type ActionDescriptor } from "./primitives";

/** The refresh kinds the Host can report, taken straight from the protocol so the two cannot drift. */
export type RefreshStatusKind = RefreshStatus["kind"];

export type RefreshStatusView = {
  kind: RefreshStatusKind;
  /** Already-composed status line: state, data as-of time, and any retry timing. */
  message: string;
  /** Recovery actions the calling page allows right now; this banner never adds its own. */
  actions: ActionDescriptor[];
};

/**
 * Tracker refresh state for one Project. The calling page resolves the Host status and the
 * recovery actions, so this banner never refreshes or decides what a user may do.
 */
export function refreshStatus(view: RefreshStatusView): string {
  return `<div class="refresh-bar" data-kind="${escapeHtml(view.kind)}">
    <span>${escapeHtml(view.message)}</span>
    ${view.actions.map((action) => button(action, { className: "refresh-action" })).join("")}
  </div>`;
}
