/**
 * ToolCallCard — displays a tool invocation with name, args, and result.
 */

import { useState } from "react";
import ReasoningCard from "./ReasoningCard";

interface ToolCallCardProps {
  stepId: string;
  timestamp: number;
  tool: string;
  args: unknown;
  result?: {
    ok: boolean;
    summary: string;
    duration_ms: number;
  };
}

export default function ToolCallCard({ stepId, timestamp, tool, args, result }: ToolCallCardProps) {
  const [expanded, setExpanded] = useState(false);

  return (
    <ReasoningCard stepId={stepId} timestamp={timestamp} variant={result ? "tool-result" : "tool-call"}>
      <div className="flex items-center gap-2 mb-1">
        <code className="text-sm font-mono text-amber-400">{tool}</code>
        {result && (
          <span className={`text-xs px-1.5 py-0.5 rounded ${result.ok ? "bg-green-900/50 text-green-400" : "bg-red-900/50 text-red-400"}`}>
            {result.ok ? "ok" : "error"} — {result.duration_ms}ms
          </span>
        )}
      </div>

      {result && (
        <p className="text-sm text-gray-400">{result.summary}</p>
      )}

      <button
        onClick={() => setExpanded(!expanded)}
        className="text-xs text-gray-500 hover:text-gray-300 mt-1"
      >
        {expanded ? "hide args" : "show args"}
      </button>

      {expanded && (
        <pre className="text-xs text-gray-500 mt-1 overflow-x-auto max-h-40 overflow-y-auto bg-gray-900 rounded p-2">
          {JSON.stringify(args, null, 2)}
        </pre>
      )}
    </ReasoningCard>
  );
}
