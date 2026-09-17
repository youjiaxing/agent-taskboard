import { invoke, isTauri } from "@tauri-apps/api/core";
import { ui } from "./ui";

export type EditMenuSurface = "none" | "editable-text" | "readonly-selection" | "terminal";

export type EditMenuContext = {
  surface: EditMenuSurface;
  canUndo: boolean;
  canRedo: boolean;
  hasSelection: boolean;
  hasContent: boolean;
  canPaste: boolean;
};

type EditMenuAction = "undo" | "redo" | "cut" | "copy" | "paste" | "select-all";

const TEXTUAL_INPUT_TYPES = new Set([
  "text",
  "search",
  "url",
  "tel",
  "email",
  "password",
  "number",
  "date",
  "datetime-local",
  "month",
  "week",
  "time",
]);

let hostWindowVisible = true;
let lastContextJson = "";
let syncTimer = 0;
let selectionHooked = false;

function nativeShellAvailable(): boolean {
  return isTauri() || "__TAURI_INTERNALS__" in window;
}

function terminalFocused(): boolean {
  const active = document.activeElement as HTMLElement | null;
  return Boolean(active?.closest(".pty-host"));
}

function textualInput(element: HTMLInputElement): boolean {
  return TEXTUAL_INPUT_TYPES.has((element.type || "text").toLowerCase());
}

function editableElement(element: HTMLElement | null): HTMLElement | null {
  if (!element) return null;
  if (element.isContentEditable && element.contentEditable !== "false") return element;
  if (element instanceof HTMLTextAreaElement && !element.disabled && !element.readOnly) return element;
  if (element instanceof HTMLInputElement && !element.disabled && !element.readOnly && textualInput(element)) {
    return element;
  }
  return null;
}

function readonlyTextElement(element: HTMLElement | null): HTMLElement | null {
  if (!element) return null;
  if (element instanceof HTMLTextAreaElement && (element.disabled || element.readOnly)) return element;
  if (element instanceof HTMLInputElement && textualInput(element) && (element.disabled || element.readOnly)) {
    return element;
  }
  return null;
}

function fieldSelection(element: HTMLElement): { hasSelection: boolean; hasContent: boolean } {
  if (element instanceof HTMLInputElement || element instanceof HTMLTextAreaElement) {
    const start = element.selectionStart ?? 0;
    const end = element.selectionEnd ?? 0;
    return { hasSelection: start !== end, hasContent: element.value.length > 0 };
  }
  const text = element.innerText ?? element.textContent ?? "";
  const selection = window.getSelection();
  const hasSelection = Boolean(
    selection
    && !selection.isCollapsed
    && selection.rangeCount > 0
    && element.contains(selection.anchorNode),
  );
  return { hasSelection, hasContent: text.trim().length > 0 };
}

function documentHasReadonlySelection(): boolean {
  const selection = window.getSelection();
  return Boolean(selection && !selection.isCollapsed && selection.toString().length > 0);
}

function queryEnabled(command: string): boolean {
  try {
    return document.queryCommandEnabled(command);
  } catch {
    return false;
  }
}

function terminalHasContent(): boolean {
  const buffer = ui.term?.buffer.active;
  if (!buffer) return false;
  if (buffer.length > 1) return true;
  return Boolean(buffer.getLine(0)?.translateToString(true).trim());
}

export function collectEditMenuContext(): EditMenuContext {
  if (!hostWindowVisible) {
    return {
      surface: "none",
      canUndo: false,
      canRedo: false,
      hasSelection: false,
      hasContent: false,
      canPaste: false,
    };
  }
  if (terminalFocused() && ui.term) {
    return {
      surface: "terminal",
      canUndo: false,
      canRedo: false,
      hasSelection: ui.term.hasSelection(),
      hasContent: terminalHasContent(),
      canPaste: true,
    };
  }
  const active = document.activeElement as HTMLElement | null;
  const editable = editableElement(active);
  if (editable) {
    const { hasSelection, hasContent } = fieldSelection(editable);
    return {
      surface: "editable-text",
      canUndo: queryEnabled("undo"),
      canRedo: queryEnabled("redo"),
      hasSelection,
      hasContent,
      canPaste: true,
    };
  }
  const readonlyField = readonlyTextElement(active);
  if (readonlyField) {
    const { hasSelection } = fieldSelection(readonlyField);
    if (hasSelection) {
      return {
        surface: "readonly-selection",
        canUndo: false,
        canRedo: false,
        hasSelection: true,
        hasContent: false,
        canPaste: false,
      };
    }
  }
  if (documentHasReadonlySelection()) {
    return {
      surface: "readonly-selection",
      canUndo: false,
      canRedo: false,
      hasSelection: true,
      hasContent: false,
      canPaste: false,
    };
  }
  return {
    surface: "none",
    canUndo: false,
    canRedo: false,
    hasSelection: false,
    hasContent: false,
    canPaste: false,
  };
}

export function scheduleEditMenuContextSync(): void {
  if (!nativeShellAvailable()) return;
  if (syncTimer) cancelAnimationFrame(syncTimer);
  syncTimer = requestAnimationFrame(() => {
    syncTimer = 0;
    void syncEditMenuContext();
  });
}

export async function syncEditMenuContext(): Promise<void> {
  if (!nativeShellAvailable()) return;
  const context = collectEditMenuContext();
  const serialized = JSON.stringify(context);
  if (serialized === lastContextJson) return;
  lastContextJson = serialized;
  try {
    await invoke("set_edit_menu_context", { context });
  } catch {
    lastContextJson = "";
  }
}

function insertPlainText(text: string): void {
  const active = document.activeElement;
  if (active instanceof HTMLInputElement || active instanceof HTMLTextAreaElement) {
    const start = active.selectionStart ?? active.value.length;
    const end = active.selectionEnd ?? active.value.length;
    active.setRangeText(text, start, end, "end");
    active.dispatchEvent(new Event("input", { bubbles: true }));
    return;
  }
  document.execCommand("insertText", false, text);
}

async function pasteFallback(): Promise<void> {
  if (!navigator.clipboard?.readText) return;
  const text = await navigator.clipboard.readText();
  if (!text) return;
  if (terminalFocused() && ui.term) {
    ui.term.paste(text);
    return;
  }
  insertPlainText(text);
}

async function copyText(text: string): Promise<void> {
  if (!text) return;
  try {
    await navigator.clipboard.writeText(text);
  } catch {
    const field = document.createElement("textarea");
    field.value = text;
    document.body.append(field);
    field.select();
    document.execCommand("copy");
    field.remove();
  }
}

async function applyTerminalAction(action: EditMenuAction): Promise<void> {
  if (!ui.term) return;
  if (action === "copy") {
    await copyText(ui.term.getSelection());
    return;
  }
  if (action === "paste") {
    if (!document.execCommand("paste")) await pasteFallback();
    return;
  }
  if (action === "select-all") {
    ui.term.selectAll();
  }
}

export async function applyEditMenuAction(action: EditMenuAction): Promise<void> {
  const context = collectEditMenuContext();
  if (context.surface === "terminal") {
    await applyTerminalAction(action);
    scheduleEditMenuContextSync();
    return;
  }
  if (context.surface === "readonly-selection") {
    if (action === "copy") document.execCommand("copy");
    return;
  }
  if (context.surface !== "editable-text") return;
  const command = action === "select-all" ? "selectAll" : action;
  if (action === "paste" && !document.execCommand("paste")) {
    await pasteFallback();
  } else if (action !== "paste") {
    document.execCommand(command);
  }
  scheduleEditMenuContextSync();
}

export function hookTerminalEditMenu(term: { onSelectionChange(listener: () => void): void }): void {
  if (selectionHooked) return;
  selectionHooked = true;
  term.onSelectionChange(() => {
    scheduleEditMenuContextSync();
  });
}

export function bindNativeMenuBridge(): void {
  if (!nativeShellAvailable()) return;
  document.addEventListener("focusin", scheduleEditMenuContextSync);
  document.addEventListener("focusout", scheduleEditMenuContextSync);
  document.addEventListener("input", scheduleEditMenuContextSync, true);
  document.addEventListener("selectionchange", scheduleEditMenuContextSync);
  window.addEventListener("agent-taskboard:host-window-hidden", () => {
    hostWindowVisible = false;
    void syncEditMenuContext();
  });
  window.addEventListener("agent-taskboard:host-window-shown", () => {
    hostWindowVisible = true;
    scheduleEditMenuContextSync();
  });
  window.addEventListener("agent-taskboard:edit-undo", () => {
    void applyEditMenuAction("undo");
  });
  window.addEventListener("agent-taskboard:edit-redo", () => {
    void applyEditMenuAction("redo");
  });
  window.addEventListener("agent-taskboard:edit-cut", () => {
    void applyEditMenuAction("cut");
  });
  window.addEventListener("agent-taskboard:edit-copy", () => {
    void applyEditMenuAction("copy");
  });
  window.addEventListener("agent-taskboard:edit-paste", () => {
    void applyEditMenuAction("paste");
  });
  window.addEventListener("agent-taskboard:edit-select-all", () => {
    void applyEditMenuAction("select-all");
  });
  scheduleEditMenuContextSync();
}
