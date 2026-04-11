/**
 * Themes — CSS token editor with live preview.
 */

import { useState } from "react";

interface ThemeTokens {
  name: string;
  bg: string;
  surface: string;
  text: string;
  accent: string;
  border: string;
}

const PRESETS: ThemeTokens[] = [
  { name: "Dark", bg: "#1a1a2e", surface: "#16213e", text: "#e2e8f0", accent: "#6366f1", border: "#374151" },
  { name: "Light", bg: "#f8fafc", surface: "#ffffff", text: "#1e293b", accent: "#4f46e5", border: "#e2e8f0" },
  { name: "Nord", bg: "#2e3440", surface: "#3b4252", text: "#eceff4", accent: "#88c0d0", border: "#4c566a" },
  { name: "Solarized", bg: "#002b36", surface: "#073642", text: "#839496", accent: "#268bd2", border: "#586e75" },
];

export default function Themes() {
  const [tokens, setTokens] = useState<ThemeTokens>(PRESETS[0]);

  const applyPreset = (preset: ThemeTokens) => {
    setTokens(preset);
    // TODO: Apply to CSS custom properties + persist via Tauri command
  };

  return (
    <div>
      <h3 className="text-lg font-semibold mb-4">Theme</h3>

      {/* Presets */}
      <div className="flex gap-2 mb-6">
        {PRESETS.map((p) => (
          <button
            key={p.name}
            onClick={() => applyPreset(p)}
            className={`px-3 py-1.5 text-sm rounded transition-colors ${
              tokens.name === p.name
                ? "bg-indigo-700 text-white"
                : "bg-gray-800 text-gray-400 hover:text-white"
            }`}
          >
            {p.name}
          </button>
        ))}
      </div>

      {/* Token editor */}
      <div className="grid grid-cols-2 gap-3 mb-6">
        {(["bg", "surface", "text", "accent", "border"] as const).map((key) => (
          <div key={key}>
            <label className="block text-xs text-gray-400 mb-1">{key}</label>
            <div className="flex items-center gap-2">
              <input
                type="color"
                value={tokens[key]}
                onChange={(e) => setTokens({ ...tokens, [key]: e.target.value })}
                className="w-8 h-8 rounded cursor-pointer border-0"
              />
              <input
                value={tokens[key]}
                onChange={(e) => setTokens({ ...tokens, [key]: e.target.value })}
                className="flex-1 bg-gray-900 border border-gray-700 rounded px-2 py-1 text-xs text-gray-300 font-mono"
              />
            </div>
          </div>
        ))}
      </div>

      {/* Live preview */}
      <h4 className="text-sm font-medium text-gray-400 mb-2">Preview</h4>
      <div
        className="rounded p-4 border"
        style={{ backgroundColor: tokens.bg, color: tokens.text, borderColor: tokens.border }}
      >
        <div className="rounded p-3 mb-2" style={{ backgroundColor: tokens.surface }}>
          <span className="text-sm font-medium">Surface element</span>
        </div>
        <span style={{ color: tokens.accent }} className="text-sm font-medium">Accent text</span>
        <span className="text-sm ml-4">Regular text</span>
      </div>
    </div>
  );
}
