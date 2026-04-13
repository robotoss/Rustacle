/**
 * Terminal state management: tabs, active tab, split layout.
 *
 * Uses a reducer pattern matching agent.ts conventions.
 */

export interface TabInfo {
  id: string;
  cwd: string;
  title: string;
  alive: boolean;
  index: number;
  active: boolean;
}

/** Recursive split layout tree from the backend. */
export type SplitLayout =
  | { kind: "leaf"; tab_id: string }
  | { kind: "split"; id: string; direction: string; ratio: number; children: SplitLayout[] };

export interface TerminalState {
  tabs: TabInfo[];
  activeTabId: string | null;
  layout: SplitLayout | null;
}

export const initialTerminalState: TerminalState = {
  tabs: [],
  activeTabId: null,
  layout: null,
};

export type TerminalAction =
  | { type: "SET_TABS"; tabs: TabInfo[] }
  | { type: "ADD_TAB"; tab: TabInfo }
  | { type: "CLOSE_TAB"; tabId: string }
  | { type: "SET_ACTIVE"; tabId: string }
  | { type: "UPDATE_LAYOUT"; layout: SplitLayout | null }
  | { type: "REORDER"; tabId: string; newIndex: number }
  | { type: "SET_TAB_TITLE"; tabId: string; title: string };

export function terminalReducer(state: TerminalState, action: TerminalAction): TerminalState {
  switch (action.type) {
    case "SET_TABS":
      return {
        ...state,
        tabs: action.tabs,
        activeTabId: action.tabs.find((t) => t.active)?.id ?? state.activeTabId,
      };

    case "ADD_TAB":
      return {
        ...state,
        tabs: [...state.tabs, action.tab],
        activeTabId: action.tab.id,
      };

    case "CLOSE_TAB": {
      const remaining = state.tabs.filter((t) => t.id !== action.tabId);
      const newActive =
        state.activeTabId === action.tabId
          ? remaining[remaining.length - 1]?.id ?? null
          : state.activeTabId;
      return { ...state, tabs: remaining, activeTabId: newActive };
    }

    case "SET_ACTIVE":
      return { ...state, activeTabId: action.tabId };

    case "UPDATE_LAYOUT":
      return { ...state, layout: action.layout };

    case "REORDER": {
      const tabs = [...state.tabs];
      const oldIdx = tabs.findIndex((t) => t.id === action.tabId);
      if (oldIdx === -1) return state;
      const [moved] = tabs.splice(oldIdx, 1);
      tabs.splice(action.newIndex, 0, moved);
      return { ...state, tabs: tabs.map((t, i) => ({ ...t, index: i })) };
    }

    case "SET_TAB_TITLE":
      return {
        ...state,
        tabs: state.tabs.map((t) =>
          t.id === action.tabId ? { ...t, title: action.title } : t
        ),
      };

    default:
      return state;
  }
}
