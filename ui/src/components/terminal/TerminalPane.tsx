import { useEffect, useRef } from "react";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { WebglAddon } from "@xterm/addon-webgl";
import "@xterm/xterm/css/xterm.css";
import { useTerminal } from "./useTerminal";

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

interface TerminalPaneProps {
  tabId: string;
  onFocus?: () => void;
  onTitle?: (title: string) => void;
}

/**
 * A single terminal pane backed by a PTY session.
 * Manages its own xterm.js instance and polls for output.
 */
export default function TerminalPane({ tabId, onFocus, onTitle }: TerminalPaneProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const { writePty, readPty, resizePty } = useTerminal();

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

    try {
      term.loadAddon(new WebglAddon());
    } catch {
      // Canvas fallback
    }

    fit.fit();
    resizePty(tabId, term.cols, term.rows);

    // Poll for output
    const poll = window.setInterval(async () => {
      try {
        const data = await readPty(tabId);
        if (data.length > 0) term.write(data);
      } catch {
        // Tab may have closed
      }
    }, 50);

    // Keystrokes → PTY
    const dataDisposable = term.onData((data) => writePty(tabId, data));

    // Title
    const titleDisposable = term.onTitleChange((title) => onTitle?.(title));

    // Resize
    const resizeObserver = new ResizeObserver(() => {
      fit.fit();
      resizePty(tabId, term.cols, term.rows);
    });
    resizeObserver.observe(containerRef.current);

    return () => {
      clearInterval(poll);
      dataDisposable.dispose();
      titleDisposable.dispose();
      resizeObserver.disconnect();
      term.dispose();
    };
  }, [tabId]); // eslint-disable-line react-hooks/exhaustive-deps

  return (
    <div
      ref={containerRef}
      className="h-full w-full bg-[#1a1a2e]"
      onFocus={onFocus}
      tabIndex={-1}
    />
  );
}
