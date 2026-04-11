import { useCallback } from "react";
import { commands } from "../../../bindings";

const TERMINAL_PLUGIN_ID = "rustacle.terminal";

/** Hook for terminal PTY operations via plugin_call. */
export function useTerminal() {
  const openTab = useCallback(async (cwd?: string) => {
    const res = await commands.pluginCall({
      plugin_id: TERMINAL_PLUGIN_ID,
      command: "open_tab",
      payload: JSON.stringify({ cwd: cwd ?? null }),
    });
    if (res.status === "ok") {
      return JSON.parse(res.data.data) as { tab_id: string };
    }
    throw new Error(`open_tab failed: ${JSON.stringify(res.error)}`);
  }, []);

  const writePty = useCallback(async (tabId: string, data: string) => {
    await commands.pluginCall({
      plugin_id: TERMINAL_PLUGIN_ID,
      command: "write",
      payload: JSON.stringify({ tab_id: tabId, data }),
    });
  }, []);

  const readPty = useCallback(async (tabId: string): Promise<Uint8Array> => {
    const res = await commands.pluginCall({
      plugin_id: TERMINAL_PLUGIN_ID,
      command: "read",
      payload: JSON.stringify({ tab_id: tabId }),
    });
    if (res.status === "ok") {
      // Data comes as raw bytes in the response
      return new TextEncoder().encode(res.data.data);
    }
    return new Uint8Array();
  }, []);

  const resizePty = useCallback(
    async (tabId: string, cols: number, rows: number) => {
      await commands.pluginCall({
        plugin_id: TERMINAL_PLUGIN_ID,
        command: "resize",
        payload: JSON.stringify({ tab_id: tabId, cols, rows }),
      });
    },
    []
  );

  const closeTab = useCallback(async (tabId: string) => {
    await commands.pluginCall({
      plugin_id: TERMINAL_PLUGIN_ID,
      command: "close_tab",
      payload: JSON.stringify({ tab_id: tabId }),
    });
  }, []);

  return { openTab, writePty, readPty, resizePty, closeTab };
}
