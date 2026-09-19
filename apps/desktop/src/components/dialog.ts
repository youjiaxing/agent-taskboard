import { escapeHtml } from "../client-utils";
import { button, htmlAttributes, iconButton, notice, type ActionDescriptor } from "./primitives";

export type DialogTier = "confirm" | "form" | "wide";
export type DialogInitialFocus = "primary" | "cancel" | "first-field" | "none";

export type DialogModel = {
  id: string;
  tier: DialogTier;
  title: string;
  body: string;
  closeLabel: string;
  dismissible: boolean;
  initialFocus: DialogInitialFocus;
  busy?: boolean;
  actions?: string;
  className?: string;
  panelTag?: "section" | "form";
  panelAttributes?: Record<string, string | number | boolean | null | undefined>;
};

function dismissAction(label: string): ActionDescriptor {
  return { id: "dismiss-dialog", label, ariaLabel: label };
}

export function dialogDismissButton(
  label: string,
  options: { initialFocus?: boolean; disabled?: boolean } = {},
): string {
  return button(
    { ...dismissAction(label), disabled: options.disabled },
    { attributes: { "data-dialog-dismiss": "true", "data-dialog-initial-focus": options.initialFocus || undefined } },
  );
}

export function dialogActionButton(
  action: ActionDescriptor,
  options: { primary?: boolean; type?: "button" | "submit"; initialFocus?: boolean } = {},
): string {
  return button(action, {
    type: options.type,
    variant: action.destructive ? "danger" : options.primary ? "primary" : "secondary",
    attributes: { "data-dialog-initial-focus": options.initialFocus || undefined },
  });
}

export function dialog(model: DialogModel): string {
  const titleId = `dialog-title-${model.id}`;
  const panelTag = model.panelTag ?? "section";
  const close = model.dismissible && !model.busy
    ? iconButton(dismissAction(model.closeLabel), {
        className: "dialog-close",
        attributes: { "data-dialog-dismiss": "true" },
      })
    : "";
  const backdropAction = model.dismissible && !model.busy ? "dismiss-dialog" : undefined;
  const initialFocus = model.initialFocus === "none" ? "" : ` data-dialog-initial="${model.initialFocus}"`;
  return `<div${htmlAttributes({
    class: "dialog-backdrop",
    "data-dialog-root": "true",
    "data-dialog-id": model.id,
    "data-dialog-escape": model.dismissible && !model.busy ? "true" : "false",
    "data-act": backdropAction,
  })}>
    <${panelTag}${htmlAttributes({
      class: `dialog-panel dialog-tier-${model.tier} ${model.className ?? ""}`,
      role: "dialog",
      "aria-modal": "true",
      "aria-labelledby": titleId,
      "aria-busy": model.busy || undefined,
      tabindex: "-1",
      ...(model.panelAttributes ?? {}),
    })}${initialFocus}>
      <header class="dialog-header">
        <h2 id="${escapeHtml(titleId)}">${escapeHtml(model.title)}</h2>
        ${close}
      </header>
      <div class="dialog-content">${model.body}</div>
      ${model.actions && !model.busy ? `<footer class="dialog-actions actions">${model.actions}</footer>` : ""}
    </${panelTag}>
  </div>`;
}

export function confirmationDialog(options: {
  id: string;
  title: string;
  body: string;
  closeLabel: string;
  cancelLabel: string;
  confirm?: ActionDescriptor;
  busy?: boolean;
  error?: string;
  className?: string;
}): string {
  const body = `${options.body}${options.error ? notice({ status: "danger", role: "alert", message: options.error }) : ""}`;
  const actions = options.busy
    ? ""
    : `${dialogDismissButton(options.cancelLabel, { initialFocus: true })}${options.confirm ? dialogActionButton(options.confirm, { primary: true }) : ""}`;
  return dialog({
    id: options.id,
    tier: "confirm",
    title: options.title,
    body,
    closeLabel: options.closeLabel,
    dismissible: false,
    initialFocus: "cancel",
    busy: options.busy,
    actions,
    className: options.className,
  });
}

export function drawer(options: {
  id: string;
  title: string;
  body: string;
  closeLabel: string;
  className?: string;
}): string {
  const titleId = `drawer-title-${options.id}`;
  return `<div class="drawer-backdrop" data-dialog-root="true" data-dialog-id="${escapeHtml(options.id)}" data-dialog-escape="true" data-act="dismiss-dialog">
    <aside class="drawer-panel ${escapeHtml(options.className ?? "")}" role="dialog" aria-modal="true" aria-labelledby="${escapeHtml(titleId)}" tabindex="-1" data-dialog-initial="first-field">
      <header class="dialog-header">
        <h2 id="${escapeHtml(titleId)}">${escapeHtml(options.title)}</h2>
        ${iconButton(dismissAction(options.closeLabel), { className: "dialog-close", attributes: { "data-dialog-dismiss": "true" } })}
      </header>
      <div class="dialog-content">${options.body}</div>
    </aside>
  </div>`;
}
