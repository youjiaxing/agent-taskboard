const triggerSelectors = new Map<string, string>();

function escaped(value: string): string {
  return CSS.escape(value);
}

function focusSelector(element: HTMLElement, fallback?: string): string | null {
  if (fallback) return fallback;
  if (element.id) return `#${escaped(element.id)}`;
  const action = element.dataset.act;
  if (action) {
    const id = element.dataset.id;
    return `[data-act="${escaped(action)}"]${id ? `[data-id="${escaped(id)}"]` : ""}`;
  }
  return null;
}

export function rememberDialogTrigger(dialogId: string, target: HTMLElement, fallback?: string): void {
  const selector = focusSelector(target, fallback);
  if (selector) triggerSelectors.set(dialogId, selector);
}

export function restoreDialogTrigger(dialogId: string): void {
  const selector = triggerSelectors.get(dialogId);
  triggerSelectors.delete(dialogId);
  if (!selector) return;
  document.querySelector<HTMLElement>(selector)?.focus();
}

function topDialogRoot(): HTMLElement | null {
  const roots = document.querySelectorAll<HTMLElement>("[data-dialog-root='true']");
  return roots.item(roots.length - 1);
}

function focusableElements(panel: HTMLElement): HTMLElement[] {
  return [...panel.querySelectorAll<HTMLElement>(
    "button:not([disabled]), input:not([disabled]), textarea:not([disabled]), select:not([disabled]), [href], [tabindex]:not([tabindex='-1'])",
  )].filter((element) => !element.hidden && element.getAttribute("aria-hidden") !== "true");
}

export function syncDialogFocus(): void {
  const root = topDialogRoot();
  if (!root) return;
  const panel = root.querySelector<HTMLElement>("[role='dialog']");
  if (!panel || panel.contains(document.activeElement)) return;
  const initial = panel.querySelector<HTMLElement>("[data-dialog-initial-focus]:not([disabled])");
  const requested = panel.dataset.dialogInitial;
  const firstField = requested === "first-field"
    ? panel.querySelector<HTMLElement>(".dialog-content input:not([disabled]), .dialog-content textarea:not([disabled]), .dialog-content select:not([disabled]), .dialog-content button:not([disabled])")
    : null;
  (initial ?? firstField ?? focusableElements(panel)[0] ?? panel).focus();
}

export function handleDialogKeydown(event: KeyboardEvent): boolean {
  const root = topDialogRoot();
  if (!root) return false;
  const panel = root.querySelector<HTMLElement>("[role='dialog']");
  if (!panel) return false;
  if (event.key === "Escape") {
    event.preventDefault();
    event.stopPropagation();
    if (root.dataset.dialogEscape === "true") {
      panel.querySelector<HTMLButtonElement>("button[data-dialog-dismiss='true']")?.click();
    }
    return true;
  }
  if (event.key !== "Tab") return false;
  const focusable = focusableElements(panel);
  if (!focusable.length) {
    event.preventDefault();
    panel.focus();
    return true;
  }
  const current = focusable.indexOf(document.activeElement as HTMLElement);
  const next = event.shiftKey
    ? current <= 0 ? focusable.length - 1 : current - 1
    : current < 0 || current === focusable.length - 1 ? 0 : current + 1;
  event.preventDefault();
  focusable[next]?.focus();
  return true;
}
