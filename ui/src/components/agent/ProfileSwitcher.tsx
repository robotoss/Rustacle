/**
 * ProfileSwitcher — compact dropdown for switching model profiles on the fly.
 * Reactively updates when settings change via `settings:changed` event.
 */

import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { commands, type ProfileSummary } from "../../../bindings";

interface ProfileSwitcherProps {
  currentProfile: string | null;
  onChange: (profile: string) => void;
}

export default function ProfileSwitcher({ currentProfile, onChange }: ProfileSwitcherProps) {
  const [profiles, setProfiles] = useState<ProfileSummary[]>([]);

  const loadProfiles = useCallback(() => {
    commands
      .listModelProfiles()
      .then((res) => {
        if (res.status === "ok") {
          setProfiles(res.data.profiles);
        }
      })
      .catch(() => {});
  }, []);

  // Load on mount.
  useEffect(() => {
    loadProfiles();
  }, [loadProfiles]);

  // Reload when any setting changes (filter for model.profiles key).
  useEffect(() => {
    const unlisten = listen("settings:changed", (event) => {
      const payload = event.payload as { key?: string };
      if (!payload.key || payload.key === "model.profiles") {
        loadProfiles();
      }
    });
    return () => { unlisten.then((f) => f()); };
  }, [loadProfiles]);

  const handleChange = useCallback(
    (e: React.ChangeEvent<HTMLSelectElement>) => {
      onChange(e.target.value);
    },
    [onChange]
  );

  if (profiles.length === 0) {
    return (
      <span className="text-xs text-gray-600 truncate max-w-[100px]">
        No profiles
      </span>
    );
  }

  return (
    <select
      value={currentProfile ?? profiles[0]?.name ?? ""}
      onChange={handleChange}
      className="bg-gray-800 text-gray-300 text-xs rounded border border-gray-600 px-1 py-0.5 max-w-[120px] truncate focus:border-blue-500 focus:outline-none"
    >
      {profiles.map((p) => (
        <option key={p.name} value={p.name}>
          {p.name} ({p.provider})
        </option>
      ))}
    </select>
  );
}
