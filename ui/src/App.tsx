import { useEffect, useState } from "react";
import { commands } from "../bindings";
import PingButton from "./components/common/PingButton";
import PluginDemo from "./components/common/PluginDemo";
import TerminalTab from "./components/terminal/Tab";

type View = "home" | "terminal";

export default function App() {
  const [appVersion, setAppVersion] = useState("");
  const [view, setView] = useState<View>("home");

  useEffect(() => {
    commands.version().then(setAppVersion);
  }, []);

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
      </main>
    </div>
  );
}
