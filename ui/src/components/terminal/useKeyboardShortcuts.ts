import { useEffect } from "react";

interface ShortcutActions {
  newTab: () => void;
  closeTab: () => void;
  splitHorizontal: () => void;
  splitVertical: () => void;
  jumpToTab: (index: number) => void;
}

/**
 * Registers global keyboard shortcuts for terminal tab management.
 *
 * - Ctrl+T — new tab
 * - Ctrl+W — close active tab
 * - Ctrl+D — split horizontal
 * - Ctrl+Shift+D — split vertical
 * - Ctrl+1..9 — jump to tab by index
 */
export function useKeyboardShortcuts(actions: ShortcutActions) {
  useEffect(() => {
    function handler(e: KeyboardEvent) {
      const ctrl = e.ctrlKey || e.metaKey;
      if (!ctrl) return;

      switch (e.key) {
        case "t":
          e.preventDefault();
          actions.newTab();
          break;
        case "w":
          e.preventDefault();
          actions.closeTab();
          break;
        case "d":
          e.preventDefault();
          if (e.shiftKey) {
            actions.splitVertical();
          } else {
            actions.splitHorizontal();
          }
          break;
        default:
          // Ctrl+1..9
          if (e.key >= "1" && e.key <= "9") {
            e.preventDefault();
            actions.jumpToTab(parseInt(e.key, 10) - 1);
          }
      }
    }

    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [actions]);
}
