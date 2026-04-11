/**
 * ImportExport — file drop zone, diff preview, selective apply.
 * Secrets are excluded from export.
 */

import { useState } from "react";

interface DiffEntry {
  key: string;
  current: unknown;
  incoming: unknown;
  changed: boolean;
  selected: boolean;
}

export default function ImportExport() {
  const [diffs, setDiffs] = useState<DiffEntry[]>([]);
  const [exported, setExported] = useState<string>("");

  const handleExport = () => {
    // TODO: Call Tauri command to export settings
    const mockExport = {
      schema_version: 1,
      settings: [
        { key: "ui.theme", value: "dark" },
        { key: "agent.max_tool_calls", value: 50 },
      ],
    };
    setExported(JSON.stringify(mockExport, null, 2));
  };

  const handleFileSelect = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;

    const reader = new FileReader();
    reader.onload = (ev) => {
      try {
        const payload = JSON.parse(ev.target?.result as string);
        // TODO: Call Tauri command to compute diff
        const mockDiffs: DiffEntry[] = (payload.settings || []).map((s: { key: string; value: unknown }) => ({
          key: s.key,
          current: "...",
          incoming: s.value,
          changed: true,
          selected: true,
        }));
        setDiffs(mockDiffs);
      } catch {
        alert("Invalid settings file");
      }
    };
    reader.readAsText(file);
  };

  const toggleEntry = (key: string) => {
    setDiffs(diffs.map((d) => (d.key === key ? { ...d, selected: !d.selected } : d)));
  };

  const handleApply = () => {
    const selected = diffs.filter((d) => d.selected && d.changed);
    // TODO: Call Tauri command to apply selected settings
    alert(`Would apply ${selected.length} settings`);
    setDiffs([]);
  };

  return (
    <div>
      <h3 className="text-lg font-semibold mb-4">Import / Export</h3>

      {/* Export */}
      <div className="mb-6">
        <h4 className="text-sm font-medium text-gray-400 mb-2">Export</h4>
        <p className="text-xs text-gray-500 mb-2">
          Export all settings (excluding API keys and secrets) as a portable file.
        </p>
        <button
          onClick={handleExport}
          className="px-4 py-1.5 text-sm bg-indigo-700 hover:bg-indigo-600 text-white rounded transition-colors"
        >
          Export Settings
        </button>
        {exported && (
          <pre className="mt-2 bg-gray-900 border border-gray-700 rounded p-3 text-xs text-gray-400 max-h-40 overflow-auto">
            {exported}
          </pre>
        )}
      </div>

      {/* Import */}
      <div>
        <h4 className="text-sm font-medium text-gray-400 mb-2">Import</h4>
        <p className="text-xs text-gray-500 mb-2">
          Import settings from a file. A diff preview is shown before any changes are applied.
        </p>
        <label className="block w-full border-2 border-dashed border-gray-700 rounded-lg p-6 text-center cursor-pointer hover:border-indigo-500 transition-colors">
          <span className="text-sm text-gray-500">Drop a settings file or click to browse</span>
          <input type="file" accept=".json" onChange={handleFileSelect} className="hidden" />
        </label>

        {/* Diff preview */}
        {diffs.length > 0 && (
          <div className="mt-4">
            <h4 className="text-sm font-medium text-gray-400 mb-2">Changes to apply</h4>
            <div className="space-y-1 mb-3">
              {diffs.filter((d) => d.changed).map((d) => (
                <label key={d.key} className="flex items-center gap-2 bg-gray-800 rounded px-3 py-2 text-sm">
                  <input
                    type="checkbox"
                    checked={d.selected}
                    onChange={() => toggleEntry(d.key)}
                    className="rounded"
                  />
                  <code className="text-amber-400">{d.key}</code>
                  <span className="text-gray-500 ml-auto text-xs">{JSON.stringify(d.incoming)}</span>
                </label>
              ))}
            </div>
            <div className="flex gap-2">
              <button
                onClick={handleApply}
                className="px-4 py-1.5 text-sm bg-green-700 hover:bg-green-600 text-white rounded transition-colors"
              >
                Apply Selected
              </button>
              <button
                onClick={() => setDiffs([])}
                className="px-4 py-1.5 text-sm text-gray-400 hover:text-white transition-colors"
              >
                Cancel
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
