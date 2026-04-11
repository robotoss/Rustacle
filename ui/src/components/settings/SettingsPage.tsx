/**
 * SettingsPage — full-page layout with sidebar navigation.
 * Zero-JSON: every setting has a UI control.
 */

import { useState } from "react";
import ModelProfiles from "./ModelProfiles";
import Permissions from "./Permissions";
import Keybindings from "./Keybindings";
import Themes from "./Themes";
import ImportExport from "./ImportExport";

type Section = "profiles" | "permissions" | "keybindings" | "themes" | "import-export";

const SECTIONS: { id: Section; label: string }[] = [
  { id: "profiles", label: "Model Profiles" },
  { id: "permissions", label: "Permissions" },
  { id: "keybindings", label: "Keybindings" },
  { id: "themes", label: "Themes" },
  { id: "import-export", label: "Import / Export" },
];

export default function SettingsPage() {
  const [active, setActive] = useState<Section>("profiles");

  return (
    <div className="flex h-full">
      {/* Sidebar */}
      <nav className="w-48 bg-gray-900 border-r border-gray-700 py-4 flex-shrink-0">
        <h2 className="px-4 text-xs text-gray-500 uppercase tracking-wide mb-3">Settings</h2>
        {SECTIONS.map((s) => (
          <button
            key={s.id}
            onClick={() => setActive(s.id)}
            className={`w-full text-left px-4 py-2 text-sm transition-colors ${
              active === s.id
                ? "bg-indigo-700/30 text-white border-r-2 border-indigo-500"
                : "text-gray-400 hover:text-white hover:bg-gray-800"
            }`}
          >
            {s.label}
          </button>
        ))}
      </nav>

      {/* Content */}
      <main className="flex-1 overflow-y-auto p-6">
        {active === "profiles" && <ModelProfiles />}
        {active === "permissions" && <Permissions />}
        {active === "keybindings" && <Keybindings />}
        {active === "themes" && <Themes />}
        {active === "import-export" && <ImportExport />}
      </main>
    </div>
  );
}
