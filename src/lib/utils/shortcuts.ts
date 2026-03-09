/**
 * Keyboard shortcut definitions and matching utility.
 */

export interface ShortcutDef {
  key: string;
  metaKey?: boolean;
  shiftKey?: boolean;
  altKey?: boolean;
  action: string;
  label: string;
}

export const SHORTCUTS: ShortcutDef[] = [
  { key: "o", metaKey: true, action: "open-folder", label: "Open Folder" },
  { key: "r", metaKey: true, action: "refresh", label: "Refresh" },
  { key: "c", metaKey: true, action: "copy-path", label: "Copy Path" },
  { key: "i", metaKey: true, action: "show-info", label: "Show Info" },
  { key: "Backspace", action: "move-to-trash", label: "Move to Trash" },
  { key: "Delete", action: "move-to-trash", label: "Move to Trash" },
  { key: "F5", action: "refresh", label: "Refresh" },
  { key: "F6", action: "toggle-free-space", label: "Toggle Free Space" },
  { key: "F7", action: "toggle-unknown", label: "Toggle Unknown Space" },
  { key: "F8", action: "toggle-extensions", label: "Toggle Extension List" },
  { key: "F9", action: "toggle-treemap", label: "Toggle Treemap" },
  { key: "Escape", action: "escape", label: "Escape" },
];

/**
 * Match a keyboard event against the shortcuts array.
 * Returns the matching shortcut's action string, or null if no match.
 */
export function matchShortcut(
  e: KeyboardEvent,
  shortcuts: ShortcutDef[] = SHORTCUTS,
): string | null {
  for (const s of shortcuts) {
    if (e.key !== s.key) continue;
    if (s.metaKey && !(e.metaKey || e.ctrlKey)) continue;
    if (!s.metaKey && (e.metaKey || e.ctrlKey)) continue;
    if (s.shiftKey && !e.shiftKey) continue;
    if (s.altKey && !e.altKey) continue;
    return s.action;
  }
  return null;
}

/**
 * Check if the currently focused element is a text input where we should
 * not intercept typing-related keys (Backspace, single letters, etc.).
 */
export function isTextInputFocused(): boolean {
  const el = document.activeElement;
  if (!el) return false;
  const tag = el.tagName.toLowerCase();
  if (tag === "input" || tag === "textarea") return true;
  if ((el as HTMLElement).isContentEditable) return true;
  return false;
}
