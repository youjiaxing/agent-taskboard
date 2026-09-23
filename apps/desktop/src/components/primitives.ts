import { escapeHtml } from "../client-utils";

export type ActionDescriptor = {
  id: string;
  label: string;
  ariaLabel?: string;
  icon?: string;
  disabled?: boolean;
  busy?: boolean;
  destructive?: boolean;
  pressed?: boolean;
  data?: Record<string, string | number | undefined>;
};

export type AttributeValue = string | number | boolean | null | undefined;

export function htmlAttributes(values: Record<string, AttributeValue>): string {
  const booleanAttributes = new Set(["checked", "disabled", "hidden", "multiple", "novalidate", "open", "readonly", "required", "selected"]);
  const attributes = Object.entries(values)
    .filter(([name, value]) => value != null && !(value === false && booleanAttributes.has(name)))
    .map(([name, value]) => value === true && booleanAttributes.has(name) ? name : `${name}="${escapeHtml(String(value))}"`);
  return attributes.length ? ` ${attributes.join(" ")}` : "";
}

function actionData(action: ActionDescriptor): Record<string, AttributeValue> {
  return Object.fromEntries(
    Object.entries(action.data ?? {}).map(([name, value]) => [`data-${name}`, value]),
  );
}

/** Shared descriptor mapping so composite controls can own their own markup. */
export function actionAttributes(action: ActionDescriptor): Record<string, AttributeValue> {
  return {
    "data-act": action.id,
    "aria-label": action.ariaLabel,
    "aria-busy": action.busy || undefined,
    "aria-pressed": action.pressed,
    disabled: action.disabled || action.busy || undefined,
    ...actionData(action),
  };
}

export function button(
  action: ActionDescriptor,
  options: {
    type?: "button" | "submit";
    variant?: "primary" | "secondary" | "ghost" | "danger";
    className?: string;
    attributes?: Record<string, AttributeValue>;
  } = {},
): string {
  const variant = action.destructive ? "danger" : options.variant ?? "secondary";
  const className = ["ui-button", variant, action.pressed ? "active" : "", action.busy ? "is-busy" : "", options.className ?? ""]
    .filter(Boolean)
    .join(" ");
  const content = `${action.icon ? `<span class="ui-button-icon" aria-hidden="true">${escapeHtml(action.icon)}</span>` : ""}<span>${escapeHtml(action.label)}</span>`;
  return `<button${htmlAttributes({
    type: options.type ?? "button",
    class: className,
    ...actionAttributes(action),
    ...(options.attributes ?? {}),
  })}>${content}</button>`;
}

export function iconButton(
  action: ActionDescriptor,
  options: { className?: string; attributes?: Record<string, AttributeValue> } = {},
): string {
  return `<button${htmlAttributes({
    type: "button",
    class: ["ui-button", "ghost", "ui-icon-button", options.className ?? ""].filter(Boolean).join(" "),
    "data-act": action.id,
    "aria-label": action.ariaLabel ?? action.label,
    disabled: action.disabled || action.busy || undefined,
    ...actionData(action),
    ...(options.attributes ?? {}),
  })}><span aria-hidden="true">${escapeHtml(action.icon ?? "×")}</span><span class="sr-only">${escapeHtml(action.label)}</span></button>`;
}

export function formField(options: {
  id?: string;
  label: string;
  control: string;
  required?: boolean;
  hint?: string;
  error?: string;
  className?: string;
}): string {
  const label = options.id
    ? `<label class="label" for="${escapeHtml(options.id)}">${escapeHtml(options.label)}${options.required ? `<span aria-hidden="true"> *</span>` : ""}</label>`
    : `<div class="label">${escapeHtml(options.label)}</div>`;
  return `<div class="ui-form-field field ${escapeHtml(options.className ?? "")}">
    ${label}
    ${options.control}
    ${options.hint ? `<p class="hint">${escapeHtml(options.hint)}</p>` : ""}
    ${options.error ? notice({ status: "danger", message: options.error }) : ""}
  </div>`;
}

export function textInput(options: {
  id?: string;
  name?: string;
  value?: string;
  type?: string;
  placeholder?: string;
  required?: boolean;
  disabled?: boolean;
  readonly?: boolean;
  className?: string;
  attributes?: Record<string, AttributeValue>;
}): string {
  return `<input${htmlAttributes({
    id: options.id,
    name: options.name,
    type: options.type ?? "text",
    value: options.value ?? "",
    placeholder: options.placeholder,
    required: options.required,
    disabled: options.disabled,
    readonly: options.readonly,
    class: ["ui-input", options.className ?? ""].filter(Boolean).join(" "),
    ...(options.attributes ?? {}),
  })} />`;
}

export function textArea(options: {
  id?: string;
  name?: string;
  value?: string;
  rows?: number;
  placeholder?: string;
  required?: boolean;
  disabled?: boolean;
  className?: string;
  attributes?: Record<string, AttributeValue>;
}): string {
  return `<textarea${htmlAttributes({
    id: options.id,
    name: options.name,
    rows: options.rows,
    placeholder: options.placeholder,
    required: options.required,
    disabled: options.disabled,
    class: ["ui-textarea", options.className ?? ""].filter(Boolean).join(" "),
    ...(options.attributes ?? {}),
  })}>${escapeHtml(options.value ?? "")}</textarea>`;
}

export type SelectOption = {
  value: string;
  label: string;
  selected?: boolean;
  disabled?: boolean;
};

export function selectControl(options: {
  id?: string;
  name?: string;
  options: SelectOption[];
  required?: boolean;
  disabled?: boolean;
  multiple?: boolean;
  size?: number;
  ariaLabel?: string;
  className?: string;
  attributes?: Record<string, AttributeValue>;
}): string {
  const choices = options.options.map((option) => `<option${htmlAttributes({
    value: option.value,
    selected: option.selected,
    disabled: option.disabled,
  })}>${escapeHtml(option.label)}</option>`).join("");
  return `<select${htmlAttributes({
    id: options.id,
    name: options.name,
    required: options.required,
    disabled: options.disabled,
    multiple: options.multiple,
    size: options.size,
    "aria-label": options.ariaLabel,
    class: ["ui-select", options.className ?? ""].filter(Boolean).join(" "),
    ...(options.attributes ?? {}),
  })}>${choices}</select>`;
}

export function checkbox(options: {
  label: string;
  checked?: boolean;
  disabled?: boolean;
  attributes?: Record<string, AttributeValue>;
  className?: string;
}): string {
  return `<label class="ui-checkbox ${escapeHtml(options.className ?? "")}">
    <input${htmlAttributes({ type: "checkbox", checked: options.checked, disabled: options.disabled, ...(options.attributes ?? {}) })} />
    <span>${escapeHtml(options.label)}</span>
  </label>`;
}

export function optionGroup(options: {
  label?: string;
  actions: ActionDescriptor[];
  className?: string;
}): string {
  return `<div class="ui-option-group choices ${escapeHtml(options.className ?? "")}"${options.label ? ` role="group" aria-label="${escapeHtml(options.label)}"` : ""}>
    ${options.actions.map((action) => button(action, { variant: action.pressed ? "primary" : "secondary" })).join("")}
  </div>`;
}

export function menu(options: {
  label?: string;
  actions: ActionDescriptor[];
  className?: string;
}): string {
  return `<div class="ui-menu ${escapeHtml(options.className ?? "")}" role="menu"${options.label ? ` aria-label="${escapeHtml(options.label)}"` : ""}>
    ${options.actions.map((action) => button(action, {
      variant: "ghost",
      attributes: { role: action.pressed == null ? "menuitem" : "menuitemradio", "aria-checked": action.pressed },
    })).join("")}
  </div>`;
}

export function badge(options: { label: string; status?: "info" | "success" | "warning" | "danger" | "neutral"; className?: string }): string {
  return `<span class="ui-badge badge-${options.status ?? "neutral"} ${escapeHtml(options.className ?? "")}">${escapeHtml(options.label)}</span>`;
}

export function notice(options: {
  message: string;
  status?: "info" | "success" | "warning" | "danger" | "neutral";
  role?: "alert" | "status";
  className?: string;
  actions?: string;
  attributes?: Record<string, AttributeValue>;
}): string {
  return `<div class="ui-notice notice ${options.status === "danger" ? "bad" : options.status === "success" ? "ok" : ""} ${escapeHtml(options.className ?? "")}"${htmlAttributes({ role: options.role, ...(options.attributes ?? {}) })}>
    <p>${escapeHtml(options.message)}</p>${options.actions ?? ""}
  </div>`;
}

export function progressFeedback(options: {
  message: string;
  progress?: number | null;
  className?: string;
}): string {
  const progress = options.progress == null
    ? `<span class="progress-static" aria-hidden="true">•••</span>`
    : `<progress max="100" value="${Math.max(0, Math.min(100, options.progress))}"></progress>`;
  return `<div class="ui-progress ${escapeHtml(options.className ?? "")}" role="status" aria-live="polite">
    <p>${escapeHtml(options.message)}</p>${progress}
  </div>`;
}

export function emptyState(options: {
  title: string;
  description?: string;
  actions?: string;
  className?: string;
}): string {
  return `<div class="ui-empty-state empty ${escapeHtml(options.className ?? "")}">
    <h1>${escapeHtml(options.title)}</h1>
    ${options.description ? `<p>${escapeHtml(options.description)}</p>` : ""}
    ${options.actions ? `<div class="actions">${options.actions}</div>` : ""}
  </div>`;
}
