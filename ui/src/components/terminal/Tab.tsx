import { useCallback, useEffect, useMemo, useReducer } from "react";
import "@xterm/xterm/css/xterm.css";
import { useTerminal } from "./useTerminal";
import { initialTerminalState, terminalReducer } from "../../state/terminal";
import TabBar from "./TabBar";
import SplitTree from "./SplitTree";
import { useKeyboardShortcuts } from "./useKeyboardShortcuts";

/**
 * Top-level terminal view. Manages tab state, split layout, and keyboard
 * shortcuts. Renders a TabBar + recursive SplitTree.
 */
export default function TerminalTab() {
  const [state, dispatch] = useReducer(terminalReducer, initialTerminalState);
  const {
    openTab,
    closeTab,
    listTabs,
    splitTab,
    getLayout,
    resizeSplit,
    reorderTab,
    setActiveTab,
    setTabTitle,
  } = useTerminal();

  // Sync layout from backend.
  const refreshLayout = useCallback(async () => {
    const layout = await getLayout();
    dispatch({ type: "UPDATE_LAYOUT", layout });
  }, [getLayout]);

  // Sync tabs from backend.
  const refreshTabs = useCallback(async () => {
    const tabs = await listTabs();
    dispatch({ type: "SET_TABS", tabs });
  }, [listTabs]);

  const refresh = useCallback(async () => {
    await Promise.all([refreshTabs(), refreshLayout()]);
  }, [refreshTabs, refreshLayout]);

  // Open first tab on mount.
  useEffect(() => {
    openTab().then(() => refresh());
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  // ── Actions ──────────────────────────────────────────────────────

  const handleNewTab = useCallback(async () => {
    await openTab();
    await refresh();
  }, [openTab, refresh]);

  const handleCloseTab = useCallback(
    async (tabId: string) => {
      await closeTab(tabId);
      await refresh();
    },
    [closeTab, refresh]
  );

  const handleSelect = useCallback(
    async (tabId: string) => {
      await setActiveTab(tabId);
      dispatch({ type: "SET_ACTIVE", tabId });
    },
    [setActiveTab]
  );

  const handleReorder = useCallback(
    async (tabId: string, newIndex: number) => {
      await reorderTab(tabId, newIndex);
      dispatch({ type: "REORDER", tabId, newIndex });
    },
    [reorderTab]
  );

  const handleSplitH = useCallback(async () => {
    if (!state.activeTabId) return;
    await splitTab(state.activeTabId, "horizontal");
    await refresh();
  }, [state.activeTabId, splitTab, refresh]);

  const handleSplitV = useCallback(async () => {
    if (!state.activeTabId) return;
    await splitTab(state.activeTabId, "vertical");
    await refresh();
  }, [state.activeTabId, splitTab, refresh]);

  const handleResizeSplit = useCallback(
    async (nodeId: string, ratio: number) => {
      await resizeSplit(nodeId, ratio);
      await refreshLayout();
    },
    [resizeSplit, refreshLayout]
  );

  const handleFocusTab = useCallback(
    (tabId: string) => {
      setActiveTab(tabId);
      dispatch({ type: "SET_ACTIVE", tabId });
    },
    [setActiveTab]
  );

  const handleTitle = useCallback(
    (tabId: string, title: string) => {
      setTabTitle(tabId, title);
      dispatch({ type: "SET_TAB_TITLE", tabId, title });
    },
    [setTabTitle]
  );

  const handleJumpToTab = useCallback(
    (index: number) => {
      const tab = state.tabs[index];
      if (tab) handleSelect(tab.id);
    },
    [state.tabs, handleSelect]
  );

  const handleCloseActive = useCallback(() => {
    if (state.activeTabId) handleCloseTab(state.activeTabId);
  }, [state.activeTabId, handleCloseTab]);

  // ── Keyboard shortcuts ───────────────────────────────────────────

  const shortcuts = useMemo(
    () => ({
      newTab: handleNewTab,
      closeTab: handleCloseActive,
      splitHorizontal: handleSplitH,
      splitVertical: handleSplitV,
      jumpToTab: handleJumpToTab,
    }),
    [handleNewTab, handleCloseActive, handleSplitH, handleSplitV, handleJumpToTab]
  );

  useKeyboardShortcuts(shortcuts);

  // ── Render ───────────────────────────────────────────────────────

  return (
    <div className="flex flex-col h-full">
      <TabBar
        tabs={state.tabs}
        activeTabId={state.activeTabId}
        onSelect={handleSelect}
        onClose={handleCloseTab}
        onNew={handleNewTab}
        onReorder={handleReorder}
      />
      <div className="flex-1 overflow-hidden">
        <SplitTree
          layout={state.layout}
          activeTabId={state.activeTabId}
          onFocusTab={handleFocusTab}
          onTitle={handleTitle}
          onResizeSplit={handleResizeSplit}
        />
      </div>
    </div>
  );
}
