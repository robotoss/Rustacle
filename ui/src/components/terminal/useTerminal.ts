import { useCallback } from "react";
import { commands } from "../../../bindings";

const TERMINAL_PLUGIN_ID = "rustacle.terminal";

/** Helper to call the terminal plugin and parse a JSON response. */
async function terminalCall<T>(command: string, payload: Record<string, unknown>): Promise<T> {
  const res = await commands.pluginCall({
    plugin_id: TERMINAL_PLUGIN_ID,
    command,
    payload: JSON.stringify(payload),
  });
  if (res.status === "ok") {
    return JSON.parse(res.data.data) as T;
  }
  throw new Error(`${command} failed: ${JSON.stringify(res.error)}`);
}

/** Helper for commands that return raw bytes (read). */
async function terminalCallRaw(command: string, payload: Record<string, unknown>): Promise<Uint8Array> {
  const res = await commands.pluginCall({
    plugin_id: TERMINAL_PLUGIN_ID,
    command,
    payload: JSON.stringify(payload),
  });
  if (res.status === "ok") {
    return new TextEncoder().encode(res.data.data);
  }
  return new Uint8Array();
}

/** Helper for commands that return an empty `{}`. */
async function terminalCallVoid(command: string, payload: Record<string, unknown>): Promise<void> {
  const res = await commands.pluginCall({
    plugin_id: TERMINAL_PLUGIN_ID,
    command,
    payload: JSON.stringify(payload),
  });
  if (res.status === "error") {
    throw new Error(`${command} failed: ${JSON.stringify(res.error)}`);
  }
}

import type { SplitLayout, TabInfo } from "../../state/terminal";

/** Hook for terminal PTY operations via plugin_call. */
export function useTerminal() {
  const openTab = useCallback(
    (cwd?: string) => terminalCall<{ tab_id: string }>("open_tab", { cwd: cwd ?? null }),
    []
  );

  const writePty = useCallback(
    (tabId: string, data: string) => terminalCallVoid("write", { tab_id: tabId, data }),
    []
  );

  const readPty = useCallback(
    (tabId: string) => terminalCallRaw("read", { tab_id: tabId }),
    []
  );

  const resizePty = useCallback(
    (tabId: string, cols: number, rows: number) =>
      terminalCallVoid("resize", { tab_id: tabId, cols, rows }),
    []
  );

  const closeTab = useCallback(
    (tabId: string) => terminalCallVoid("close_tab", { tab_id: tabId }),
    []
  );

  const listTabs = useCallback(
    () => terminalCall<TabInfo[]>("list_tabs", {}),
    []
  );

  const splitTab = useCallback(
    (tabId: string, direction: "horizontal" | "vertical") =>
      terminalCall<{ new_tab_id: string; split_node_id: string }>("split_tab", {
        tab_id: tabId,
        direction,
      }),
    []
  );

  const resizeSplit = useCallback(
    (nodeId: string, ratio: number) =>
      terminalCallVoid("resize_split", { node_id: nodeId, ratio }),
    []
  );

  const getLayout = useCallback(
    () => terminalCall<SplitLayout | null>("get_layout", {}),
    []
  );

  const reorderTab = useCallback(
    (tabId: string, newIndex: number) =>
      terminalCallVoid("reorder_tab", { tab_id: tabId, new_index: newIndex }),
    []
  );

  const setActiveTab = useCallback(
    (tabId: string) => terminalCallVoid("set_active_tab", { tab_id: tabId }),
    []
  );

  const setTabTitle = useCallback(
    (tabId: string, title: string) =>
      terminalCallVoid("set_tab_title", { tab_id: tabId, title }),
    []
  );

  return {
    openTab,
    writePty,
    readPty,
    resizePty,
    closeTab,
    listTabs,
    splitTab,
    resizeSplit,
    getLayout,
    reorderTab,
    setActiveTab,
    setTabTitle,
  };
}
