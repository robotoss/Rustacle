/**
 * Permissions — per-plugin capability grants.
 * Changes invalidate the permission broker cache immediately.
 */

import { useState } from "react";

interface PluginGrant {
  plugin: string;
  capabilities: string[];
  enabled: boolean;
}

const MOCK_GRANTS: PluginGrant[] = [
  { plugin: "fs", capabilities: ["Fs(read)", "Fs(write)"], enabled: true },
  { plugin: "terminal", capabilities: ["Pty"], enabled: true },
  { plugin: "agent", capabilities: ["LlmProvider"], enabled: true },
];

export default function Permissions() {
  const [grants, setGrants] = useState<PluginGrant[]>(MOCK_GRANTS);

  const togglePlugin = (plugin: string) => {
    setGrants(grants.map((g) =>
      g.plugin === plugin ? { ...g, enabled: !g.enabled } : g
    ));
    // TODO: Persist via Tauri command → invalidate PermissionBroker cache
  };

  return (
    <div>
      <h3 className="text-lg font-semibold mb-4">Plugin Permissions</h3>
      <p className="text-sm text-gray-500 mb-4">
        Control which capabilities each plugin can use. Changes take effect immediately.
      </p>

      <div className="space-y-2">
        {grants.map((g) => (
          <div key={g.plugin} className="flex items-center justify-between bg-gray-800 rounded px-4 py-3">
            <div>
              <span className="text-sm font-medium text-gray-200">{g.plugin}</span>
              <div className="flex gap-1 mt-1">
                {g.capabilities.map((cap) => (
                  <span key={cap} className="text-xs bg-gray-700 text-gray-400 px-1.5 py-0.5 rounded">{cap}</span>
                ))}
              </div>
            </div>
            <button
              onClick={() => togglePlugin(g.plugin)}
              className={`px-3 py-1 text-xs rounded transition-colors ${
                g.enabled
                  ? "bg-green-800 text-green-300 hover:bg-green-700"
                  : "bg-red-900 text-red-400 hover:bg-red-800"
              }`}
            >
              {g.enabled ? "Enabled" : "Disabled"}
            </button>
          </div>
        ))}
      </div>
    </div>
  );
}
