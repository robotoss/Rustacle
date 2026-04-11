import { useEffect, useRef, useState } from "react";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { WebglAddon } from "@xterm/addon-webgl";
import "@xterm/xterm/css/xterm.css";
import { useTerminal } from "./useTerminal";

/** Dark theme matching Rustacle's color scheme. */
const THEME = {
  background: "#1a1a2e",
  foreground: "#e0e0e0",
  cursor: "#e0e0e0",
  cursorAccent: "#1a1a2e",
  selectionBackground: "#3a3a5e",
  black: "#1a1a2e",
  red: "#f87171",
  green: "#4ade80",
  yellow: "#facc15",
  blue: "#60a5fa",
  magenta: "#c084fc",
  cyan: "#22d3ee",
  white: "#e0e0e0",
};

interface TabProps {
  onTitle?: (title: string) => void;
}

export default function TerminalTab({ onTitle }: TabProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const termRef = useRef<Terminal | null>(null);
  const fitRef = useRef<FitAddon | null>(null);
  const tabIdRef = useRef<string | null>(null);
  const pollRef = useRef<number | null>(null);
  const { openTab, writePty, readPty, resizePty } = useTerminal();
  const [ready, setReady] = useState(false);

  useEffect(() => {
    if (!containerRef.current) return;

    const term = new Terminal({
      cursorBlink: true,
      fontSize: 14,
      fontFamily: "'Cascadia Code', 'Fira Code', monospace",
      theme: THEME,
      scrollback: 100_000,
    });

    const fit = new FitAddon();
    term.loadAddon(fit);

    term.open(containerRef.current);

    // Try WebGL, fall back to canvas
    try {
      const webgl = new WebglAddon();
      term.loadAddon(webgl);
    } catch {
      console.warn("WebGL addon failed, using canvas renderer");
    }

    fit.fit();
    termRef.current = term;
    fitRef.current = fit;

    // Spawn PTY
    openTab().then((res) => {
      tabIdRef.current = res.tab_id;
      setReady(true);

      // Resize PTY to match terminal size
      resizePty(res.tab_id, term.cols, term.rows);

      // Poll for PTY output (event-based streaming comes later)
      const poll = window.setInterval(async () => {
        if (!tabIdRef.current) return;
        try {
          const data = await readPty(tabIdRef.current);
          if (data.length > 0) {
            term.write(data);
          }
        } catch {
          // Tab may have closed
        }
      }, 50);
      pollRef.current = poll;
    });

    // Send keystrokes to PTY
    term.onData((data) => {
      if (tabIdRef.current) {
        writePty(tabIdRef.current, data);
      }
    });

    // Handle resize
    const resizeObserver = new ResizeObserver(() => {
      fit.fit();
      if (tabIdRef.current) {
        resizePty(tabIdRef.current, term.cols, term.rows);
      }
    });
    resizeObserver.observe(containerRef.current);

    // Title changes
    term.onTitleChange((title) => {
      onTitle?.(title);
    });

    return () => {
      if (pollRef.current) clearInterval(pollRef.current);
      resizeObserver.disconnect();
      term.dispose();
    };
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  return (
    <div className="flex flex-col h-full">
      <div className="bg-gray-800 px-3 py-1 text-xs text-gray-400 border-b border-gray-700 flex items-center gap-2">
        <span className="text-green-400">●</span>
        <span>{ready ? `Terminal (${tabIdRef.current})` : "Connecting..."}</span>
      </div>
      <div ref={containerRef} className="flex-1 bg-[#1a1a2e]" />
    </div>
  );
}
