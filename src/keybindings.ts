export interface KeyBinding {
  id: string;
  keys: string;
  description: string;
}

export const DEFAULT_KEYBINDINGS: KeyBinding[] = [
  { id: "save", keys: "Cmd+S", description: "Save current file" },
  { id: "toggle-sidebar", keys: "Cmd+B", description: "Toggle sidebar" },
  {
    id: "toggle-bottom-panel",
    keys: "Cmd+`",
    description: "Toggle bottom panel",
  },
  { id: "close-tab", keys: "Cmd+W", description: "Close current tab" },
  { id: "build", keys: "Cmd+Shift+B", description: "Run build" },
  { id: "new-project", keys: "Cmd+Shift+N", description: "New project" },
];

/**
 * Convert a VS Code keybinding format (e.g., "ctrl+s", "cmd+shift+k") to our internal format (e.g., "Cmd+S")
 */
export function normalizeVscodeKeybinding(vscodeKey: string): string {
  // VS Code uses lowercase "ctrl" and "cmd", we use "Cmd" and "Ctrl"
  // VS Code uses "+" separator, we also use "+"
  let normalized = vscodeKey
    .replace(/^ctrl\+/i, "Ctrl+")
    .replace(/^cmd\+/i, "Cmd+")
    .replace(/\+ctrl\+/gi, "+Ctrl+")
    .replace(/\+cmd\+/gi, "+Cmd+");

  // Capitalize first letter of key names
  const parts = normalized.split("+");
  normalized = parts
    .map((part) => {
      if (part.match(/^(cmd|ctrl|shift|alt)$/i)) {
        return part.charAt(0).toUpperCase() + part.slice(1).toLowerCase();
      }
      if (part.toLowerCase() === "enter") return "Enter";
      if (part.toLowerCase() === "escape") return "Esc";
      if (part.toLowerCase() === "backtick") return "`";
      if (part.length === 1) return part.toUpperCase();
      return part.charAt(0).toUpperCase() + part.slice(1).toLowerCase();
    })
    .join("+");

  return normalized;
}

/**
 * Parse a keybinding string into individual modifier keys and the main key
 * Returns { modifiers: Set<string>, key: string }
 */
export function parseKeybinding(keybindingStr: string): {
  modifiers: Set<string>;
  key: string;
} {
  const parts = keybindingStr.split("+");
  const modifiers = new Set<string>();
  let key = "";

  for (const part of parts) {
    const lower = part.toLowerCase();
    if (
      lower === "cmd" ||
      lower === "ctrl" ||
      lower === "shift" ||
      lower === "alt"
    ) {
      modifiers.add(lower);
    } else {
      key = lower;
    }
  }

  return { modifiers, key };
}

/**
 * Check if a KeyboardEvent matches a parsed keybinding
 */
export function keyEventMatches(
  event: KeyboardEvent,
  keybinding: { modifiers: Set<string>; key: string },
): boolean {
  const { modifiers, key } = keybinding;

  // Check modifiers
  if (
    modifiers.has("cmd") !==
    (event.metaKey ||
      (event.ctrlKey && navigator.platform.toUpperCase().indexOf("MAC") >= 0))
  ) {
    return false;
  }
  if (
    modifiers.has("ctrl") !==
    (event.ctrlKey && navigator.platform.toUpperCase().indexOf("MAC") < 0)
  ) {
    return false;
  }
  if (modifiers.has("shift") !== event.shiftKey) {
    return false;
  }
  if (modifiers.has("alt") !== event.altKey) {
    return false;
  }

  // Check key
  const eventKey = event.key.toLowerCase();
  if (key === "`" && eventKey === "`") return true;
  if (key === eventKey) return true;
  if (key === "enter" && eventKey === "enter") return true;
  if (key === "esc" && eventKey === "escape") return true;

  return false;
}
