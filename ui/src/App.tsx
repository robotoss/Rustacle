import { useCallback, useEffect, useMemo, useState } from "react";
import { commands } from "../bindings";
import PingButton from "./components/common/PingButton";
import PluginDemo from "./components/common/PluginDemo";
import TerminalTab from "./components/terminal/Tab";
import AgentPanel from "./components/agent/AgentPanel";
import SettingsPage from "./components/settings/SettingsPage";
import CommandPalette from "./components/palette/CommandPalette";
import type { PaletteItem } from "./components/palette/PaletteEntry";

type View = "home" | "terminal" | "settings";

export default function App() {
  const [appVersion, setAppVersion] = useState("");
  const [view, setView] = useState<View>("home");
  const [paletteOpen, setPaletteOpen] = useState(false);

  useEffect(() => {
    commands.version().then(setAppVersion);
  }, []);

  // Ctrl/Cmd+K toggles the command palette.
  useEffect(() => {
    function handler(e: KeyboardEvent) {
      if ((e.ctrlKey || e.metaKey) && e.key === "k") {
        e.preventDefault();
        setPaletteOpen((prev) => !prev);
      }
    }
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, []);

  const closePalette = useCallback(() => setPaletteOpen(false), []);

  const paletteEntries: PaletteItem[] = useMemo(
    () => [
      { id: "nav.home", label: "Go to Home", category: "Navigation", source: "builtin", action: () => setView("home") },
      { id: "nav.terminal", label: "Go to Terminal", shortcut: "Ctrl+T", category: "Navigation", source: "builtin", action: () => setView("terminal") },
      { id: "nav.settings", label: "Go to Settings", shortcut: "Ctrl+,", category: "Navigation", source: "builtin", action: () => setView("settings") },
      { id: "terminal.new_tab", label: "Terminal: New Tab", shortcut: "Ctrl+T", category: "Terminal", source: "builtin", action: () => { setView("terminal"); /* Tab.tsx handles Ctrl+T */ } },
      { id: "terminal.split_h", label: "Terminal: Split Horizontal", shortcut: "Ctrl+D", category: "Terminal", source: "builtin", action: () => setView("terminal") },
      { id: "terminal.split_v", label: "Terminal: Split Vertical", shortcut: "Ctrl+Shift+D", category: "Terminal", source: "builtin", action: () => setView("terminal") },
      { id: "agent.toggle", label: "Agent: Toggle Panel", shortcut: "Ctrl+J", category: "Agent", source: "builtin", action: () => { /* AgentPanel handles Ctrl+J */ } },
    ],
    []
  );

  return (
    <div className="flex flex-col h-screen bg-[#1a1a2e] text-gray-200">
      {/* Navigation bar */}
      <nav className="flex items-center gap-2 px-4 py-2 bg-gray-900 border-b border-gray-700">
        <h1 className="text-lg font-bold mr-4">Rustacle</h1>
        <button
          onClick={() => setView("home")}
          className={`px-3 py-1 rounded text-sm transition-colors ${
            view === "home"
              ? "bg-indigo-700 text-white"
              : "text-gray-400 hover:text-white"
          }`}
        >
          Home
        </button>
        <button
          onClick={() => setView("terminal")}
          className={`px-3 py-1 rounded text-sm transition-colors ${
            view === "terminal"
              ? "bg-indigo-700 text-white"
              : "text-gray-400 hover:text-white"
          }`}
        >
          Terminal
        </button>
        <button
          onClick={() => setView("settings")}
          className={`px-3 py-1 rounded text-sm transition-colors ${
            view === "settings"
              ? "bg-indigo-700 text-white"
              : "text-gray-400 hover:text-white"
          }`}
        >
          Settings
        </button>
        <span className="ml-auto text-xs text-gray-500">
          {appVersion ? `v${appVersion}` : ""}
        </span>
      </nav>

      {/* Content */}
      <main className="flex-1 overflow-hidden">
        {view === "home" && (
          <div className="flex items-center justify-center h-full">
            <div className="text-center w-full max-w-lg p-8">
              <h2 className="text-3xl font-bold mb-2">Rustacle</h2>
              <p className="text-gray-500 mb-6">Agentic Terminal</p>
              <PingButton />
              <PluginDemo />
            </div>
          </div>
        )}
        {view === "terminal" && <TerminalTab />}
        {view === "settings" && <SettingsPage />}
      </main>

      {/* Agent reasoning panel (Ctrl+J to toggle) */}
      <AgentPanel />

      {/* Command palette (Ctrl/Cmd+K) */}
      <CommandPalette open={paletteOpen} onClose={closePalette} entries={paletteEntries} />
    </div>
  );
}
