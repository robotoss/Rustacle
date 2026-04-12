/**
 * ModelProfiles — CRUD for model profiles.
 * Provider, model, endpoint, API key (via keyring), temperature, max_tokens.
 * Persisted to SQLite via get_setting / set_setting IPC commands.
 */

import { useCallback, useEffect, useState } from "react";
import { commands } from "../../../bindings";

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

const SETTING_KEY = "model.profiles";

/** Convert internal format to JSON for storage. */
function profilesToJson(profiles: ModelProfile[]): unknown[] {
  return profiles.map((p) => ({
    name: p.name,
    provider: p.provider,
    model: p.model,
    api_base: p.apiBase,
    max_tokens: p.maxTokens,
    temperature: p.temperature,
  }));
}

/** Convert stored JSON back to internal format. */
function jsonToProfiles(json: unknown[]): ModelProfile[] {
  return json.map((item) => {
    const v = item as Record<string, unknown>;
    return {
    name: (v.name as string) || "",
    provider: (v.provider as string) || "local",
    model: (v.model as string) || "",
    apiBase: (v.api_base as string) || "",
    maxTokens: (v.max_tokens as number) || 4096,
    temperature: (v.temperature as number) || 0.0,
  };
  });
}

export default function ModelProfiles() {
  const [profiles, setProfiles] = useState<ModelProfile[]>([]);
  const [editing, setEditing] = useState<ModelProfile | null>(null);
  const [isNew, setIsNew] = useState(false);
  const [saving, setSaving] = useState(false);
  const [apiKey, setApiKey] = useState("");
  const [testing, setTesting] = useState(false);
  const [testResult, setTestResult] = useState<{ ok: boolean; message: string; latency_ms: number } | null>(null);

  // Load profiles from settings store on mount.
  useEffect(() => {
    commands
      .getSetting({ key: SETTING_KEY })
      .then((res) => {
        if (res.status === "ok") {
          try {
            const parsed = JSON.parse(res.data.value_json);
            if (Array.isArray(parsed)) {
              setProfiles(jsonToProfiles(parsed));
            }
          } catch {
            // Invalid JSON — ignore, use empty
          }
        }
      })
      .catch(() => {});
  }, []);

  // Persist profiles to settings store.
  const persistProfiles = useCallback(async (updated: ModelProfile[]) => {
    setSaving(true);
    try {
      await commands.setSetting({
        key: SETTING_KEY,
        value_json: JSON.stringify(profilesToJson(updated)),
      });
    } catch {
      // Best effort — store might not be ready
    }
    setSaving(false);
  }, []);

  const handleNew = () => {
    setEditing({ ...DEFAULT_PROFILE });
    setIsNew(true);
    setApiKey("");
    setTestResult(null);
  };

  const handleEdit = (p: ModelProfile) => {
    setEditing({ ...p });
    setIsNew(false);
    setApiKey("");
    setTestResult(null);
  };

  const handleTest = async () => {
    if (!editing) return;
    setTesting(true);
    setTestResult(null);
    try {
      const res = await commands.testModelConnection({
        provider: editing.provider,
        model: editing.model,
        api_base: editing.apiBase,
        api_key: apiKey,
      });
      if (res.status === "ok") {
        setTestResult(res.data);
      } else {
        setTestResult({ ok: false, message: "IPC error", latency_ms: 0 });
      }
    } catch {
      setTestResult({ ok: false, message: "Request failed", latency_ms: 0 });
    }
    setTesting(false);
  };

  const handleSave = async () => {
    if (!editing || !editing.name.trim()) return;

    let updated: ModelProfile[];
    if (isNew) {
      updated = [...profiles, editing];
    } else {
      updated = profiles.map((p) => (p.name === editing.name ? editing : p));
    }
    setProfiles(updated);
    await persistProfiles(updated);
    setEditing(null);
  };

  const handleDelete = async (name: string) => {
    const updated = profiles.filter((p) => p.name !== name);
    setProfiles(updated);
    await persistProfiles(updated);
  };

  return (
    <div>
      <div className="flex items-center justify-between mb-4">
        <h3 className="text-lg font-semibold">Model Profiles</h3>
        <div className="flex items-center gap-2">
          {saving && <span className="text-xs text-amber-400">Saving...</span>}
          <button
            onClick={handleNew}
            className="px-3 py-1 text-sm bg-indigo-700 hover:bg-indigo-600 text-white rounded transition-colors"
          >
            + New Profile
          </button>
        </div>
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
              {p.apiBase && <span className="text-xs text-gray-600 ml-2">({p.apiBase})</span>}
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
            <label className="block text-xs text-gray-400 mb-1">API Key</label>
            <input
              type="password"
              value={apiKey}
              onChange={(e) => setApiKey(e.target.value)}
              className="w-full bg-gray-900 border border-gray-700 rounded px-3 py-1.5 text-sm text-gray-200"
              placeholder="Enter API key (used for test, stored in keyring)"
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
          {/* Test result */}
          {testResult && (
            <div className={`flex items-center gap-2 px-3 py-2 rounded text-sm ${
              testResult.ok
                ? "bg-green-900/40 border border-green-700/50 text-green-300"
                : "bg-red-900/40 border border-red-700/50 text-red-300"
            }`}>
              <span className={`inline-block w-2 h-2 rounded-full ${testResult.ok ? "bg-green-400" : "bg-red-400"}`} />
              <span>{testResult.message}</span>
              {testResult.latency_ms > 0 && (
                <span className="text-xs text-gray-500 ml-auto">{testResult.latency_ms}ms</span>
              )}
            </div>
          )}

          <div className="flex gap-2 pt-2">
            <button onClick={handleSave} className="px-4 py-1.5 text-sm bg-green-700 hover:bg-green-600 text-white rounded transition-colors">
              Save
            </button>
            <button
              onClick={handleTest}
              disabled={testing || !editing.model}
              className="px-4 py-1.5 text-sm bg-blue-700 hover:bg-blue-600 text-white rounded transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {testing ? "Testing..." : "Test Connection"}
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
