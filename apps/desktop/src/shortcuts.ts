/**
 * Shell keyboard shortcuts, owned by the Client.
 *
 * Every entry stays free of modifier keys: ⌘/Ctrl combinations belong to the OS and to the
 * official Agent TUI, and the keydown handler hands all keys to the terminal while it has focus.
 */
export type ShellShortcutId =
  | "help"
  | "search"
  | "next-card"
  | "previous-card"
  | "open-card"
  | "dismiss";

export type ShellShortcut = {
  id: ShellShortcutId;
  /** `KeyboardEvent.key` values this shortcut handles, in display order. */
  keys: string[];
};

export const SHELL_SHORTCUTS: readonly ShellShortcut[] = [
  { id: "help", keys: ["?"] },
  { id: "search", keys: ["/"] },
  { id: "next-card", keys: ["j", "J", "ArrowDown"] },
  { id: "previous-card", keys: ["k", "K", "ArrowUp"] },
  { id: "open-card", keys: ["Enter"] },
  { id: "dismiss", keys: ["Escape"] },
];

const KEY_LABELS: Record<string, string> = {
  "?": "?",
  "/": "/",
  j: "J",
  k: "K",
  ArrowDown: "↓",
  ArrowUp: "↑",
  Enter: "⏎",
  Escape: "Esc",
};

export function shortcutHandles(id: ShellShortcutId, key: string): boolean {
  return SHELL_SHORTCUTS.find((shortcut) => shortcut.id === id)?.keys.includes(key) ?? false;
}

/** Display forms of this shortcut's keys, without repeats (J and j are one chip). */
export function shortcutKeyLabels(id: ShellShortcutId): string[] {
  const keys = SHELL_SHORTCUTS.find((shortcut) => shortcut.id === id)?.keys ?? [];
  return [...new Set(keys.map((key) => KEY_LABELS[key] ?? key))];
}
