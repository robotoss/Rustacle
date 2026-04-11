/**
 * Keybindings — bundle switcher with conflict detection.
 */

import { useState } from "react";

type Bundle = "vscode" | "vim" | "emacs";

const BUNDLES: { id: Bundle; label: string; description: string }[] = [
  { id: "vscode", label: "VS Code", description: "Standard keybindings familiar to VS Code users" },
  { id: "vim", label: "Vim", description: "Modal editing with hjkl navigation" },
  { id: "emacs", label: "Emacs", description: "Emacs-style keybindings with Ctrl combinations" },
];

export default function Keybindings() {
  const [active, setActive] = useState<Bundle>("vscode");

  const handleChange = (bundle: Bundle) => {
    setActive(bundle);
    // TODO: Persist via Tauri command → rustacle-settings
  };

  return (
    <div>
      <h3 className="text-lg font-semibold mb-4">Keybindings</h3>

      <div className="space-y-2 mb-6">
        {BUNDLES.map((b) => (
          <button
            key={b.id}
            onClick={() => handleChange(b.id)}
            className={`w-full text-left px-4 py-3 rounded transition-colors ${
              active === b.id
                ? "bg-indigo-700/30 border border-indigo-500 text-white"
                : "bg-gray-800 border border-gray-700 text-gray-400 hover:text-white"
            }`}
          >
            <div className="text-sm font-medium">{b.label}</div>
            <div className="text-xs text-gray-500 mt-0.5">{b.description}</div>
          </button>
        ))}
      </div>

      <h4 className="text-sm font-medium text-gray-400 mb-2">Active Shortcuts</h4>
      <div className="bg-gray-800 rounded p-4 text-xs text-gray-500 space-y-1">
        <div className="flex justify-between"><span>Toggle Agent Panel</span><kbd className="bg-gray-700 px-1.5 py-0.5 rounded">Ctrl+J</kbd></div>
        <div className="flex justify-between"><span>Open Settings</span><kbd className="bg-gray-700 px-1.5 py-0.5 rounded">Ctrl+,</kbd></div>
        <div className="flex justify-between"><span>New Terminal Tab</span><kbd className="bg-gray-700 px-1.5 py-0.5 rounded">Ctrl+T</kbd></div>
        <div className="flex justify-between"><span>Close Tab</span><kbd className="bg-gray-700 px-1.5 py-0.5 rounded">Ctrl+W</kbd></div>
        <div className="flex justify-between"><span>Stop Agent</span><kbd className="bg-gray-700 px-1.5 py-0.5 rounded">Esc</kbd></div>
      </div>
    </div>
  );
}
