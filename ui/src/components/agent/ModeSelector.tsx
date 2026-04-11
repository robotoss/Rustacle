/**
 * ModeSelector — segmented control for Chat / Plan / Ask modes.
 */

import type { AgentMode } from "../../state/agent";

interface ModeSelectorProps {
  mode: AgentMode;
  onChange: (mode: AgentMode) => void;
  disabled: boolean;
}

const MODES: AgentMode[] = ["Chat", "Plan", "Ask"];

export default function ModeSelector({ mode, onChange, disabled }: ModeSelectorProps) {
  return (
    <div className="flex rounded overflow-hidden border border-gray-600 text-xs">
      {MODES.map((m) => (
        <button
          key={m}
          onClick={() => onChange(m)}
          disabled={disabled}
          className={`px-2 py-0.5 transition-colors ${
            mode === m
              ? "bg-blue-700 text-white"
              : "bg-gray-800 text-gray-400 hover:bg-gray-700 hover:text-gray-200"
          } disabled:opacity-50 disabled:cursor-not-allowed`}
        >
          {m}
        </button>
      ))}
    </div>
  );
}
