/**
 * ModelProfiles — CRUD for model profiles.
 * Provider, model, endpoint, API key (via keyring), temperature, max_tokens.
 */

import { useState } from "react";

interface ModelProfile {
  name: string;
  provider: string;
  model: string;
  apiBase: string;
  maxTokens: number;
  temperature: number;
}

const DEFAULT_PROFILE: ModelProfile = {
  name: "",
  provider: "local",
  model: "",
  apiBase: "",
  maxTokens: 4096,
  temperature: 0.0,
};

const PROVIDERS = ["local", "openai", "anthropic"];

export default function ModelProfiles() {
  const [profiles, setProfiles] = useState<ModelProfile[]>([]);
  const [editing, setEditing] = useState<ModelProfile | null>(null);
  const [isNew, setIsNew] = useState(false);

  const handleNew = () => {
    setEditing({ ...DEFAULT_PROFILE });
    setIsNew(true);
  };

  const handleEdit = (p: ModelProfile) => {
    setEditing({ ...p });
    setIsNew(false);
  };

  const handleSave = () => {
    if (!editing || !editing.name.trim()) return;

    if (isNew) {
      setProfiles([...profiles, editing]);
    } else {
      setProfiles(profiles.map((p) => (p.name === editing.name ? editing : p)));
    }
    // TODO: Persist via Tauri command → rustacle-settings
    setEditing(null);
  };

  const handleDelete = (name: string) => {
    setProfiles(profiles.filter((p) => p.name !== name));
    // TODO: Persist via Tauri command
  };

  return (
    <div>
      <div className="flex items-center justify-between mb-4">
        <h3 className="text-lg font-semibold">Model Profiles</h3>
        <button
          onClick={handleNew}
          className="px-3 py-1 text-sm bg-indigo-700 hover:bg-indigo-600 text-white rounded transition-colors"
        >
          + New Profile
        </button>
      </div>

      {/* Profile list */}
      {profiles.length === 0 && !editing && (
        <p className="text-gray-500 text-sm">No profiles configured. Click &quot;+ New Profile&quot; to add one.</p>
      )}

      <div className="space-y-2 mb-4">
        {profiles.map((p) => (
          <div key={p.name} className="flex items-center justify-between bg-gray-800 rounded px-4 py-3">
            <div>
              <span className="text-sm font-medium text-gray-200">{p.name}</span>
              <span className="text-xs text-gray-500 ml-2">{p.provider} / {p.model}</span>
            </div>
            <div className="flex gap-2">
              <button onClick={() => handleEdit(p)} className="text-xs text-gray-400 hover:text-white">Edit</button>
              <button onClick={() => handleDelete(p.name)} className="text-xs text-red-400 hover:text-red-300">Delete</button>
            </div>
          </div>
        ))}
      </div>

      {/* Edit form */}
      {editing && (
        <div className="bg-gray-800 rounded p-4 space-y-3">
          <div>
            <label className="block text-xs text-gray-400 mb-1">Profile Name</label>
            <input
              value={editing.name}
              onChange={(e) => setEditing({ ...editing, name: e.target.value })}
              disabled={!isNew}
              className="w-full bg-gray-900 border border-gray-700 rounded px-3 py-1.5 text-sm text-gray-200 disabled:opacity-50"
              placeholder="e.g., default, fast, local"
            />
          </div>
          <div className="grid grid-cols-2 gap-3">
            <div>
              <label className="block text-xs text-gray-400 mb-1">Provider</label>
              <select
                value={editing.provider}
                onChange={(e) => setEditing({ ...editing, provider: e.target.value })}
                className="w-full bg-gray-900 border border-gray-700 rounded px-3 py-1.5 text-sm text-gray-200"
              >
                {PROVIDERS.map((p) => <option key={p} value={p}>{p}</option>)}
              </select>
            </div>
            <div>
              <label className="block text-xs text-gray-400 mb-1">Model</label>
              <input
                value={editing.model}
                onChange={(e) => setEditing({ ...editing, model: e.target.value })}
                className="w-full bg-gray-900 border border-gray-700 rounded px-3 py-1.5 text-sm text-gray-200"
                placeholder="e.g., gpt-4o, claude-sonnet-4-20250514"
              />
            </div>
          </div>
          <div>
            <label className="block text-xs text-gray-400 mb-1">API Base URL</label>
            <input
              value={editing.apiBase}
              onChange={(e) => setEditing({ ...editing, apiBase: e.target.value })}
              className="w-full bg-gray-900 border border-gray-700 rounded px-3 py-1.5 text-sm text-gray-200"
              placeholder="e.g., http://localhost:11434/v1"
            />
          </div>
          <div>
            <label className="block text-xs text-gray-400 mb-1">API Key (stored in OS keyring)</label>
            <input
              type="password"
              className="w-full bg-gray-900 border border-gray-700 rounded px-3 py-1.5 text-sm text-gray-200"
              placeholder="Enter to update (stored securely)"
            />
          </div>
          <div className="grid grid-cols-2 gap-3">
            <div>
              <label className="block text-xs text-gray-400 mb-1">Max Tokens</label>
              <input
                type="number"
                value={editing.maxTokens}
                onChange={(e) => setEditing({ ...editing, maxTokens: Number(e.target.value) })}
                className="w-full bg-gray-900 border border-gray-700 rounded px-3 py-1.5 text-sm text-gray-200"
              />
            </div>
            <div>
              <label className="block text-xs text-gray-400 mb-1">Temperature</label>
              <input
                type="number"
                step="0.1"
                min="0"
                max="2"
                value={editing.temperature}
                onChange={(e) => setEditing({ ...editing, temperature: Number(e.target.value) })}
                className="w-full bg-gray-900 border border-gray-700 rounded px-3 py-1.5 text-sm text-gray-200"
              />
            </div>
          </div>
          <div className="flex gap-2 pt-2">
            <button onClick={handleSave} className="px-4 py-1.5 text-sm bg-green-700 hover:bg-green-600 text-white rounded transition-colors">
              Save
            </button>
            <button onClick={() => setEditing(null)} className="px-4 py-1.5 text-sm text-gray-400 hover:text-white transition-colors">
              Cancel
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
