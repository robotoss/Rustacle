/**
 * PermissionCard — blocks the turn until the user decides.
 * Shows capability, Deny / Allow-once / Allow-always buttons.
 */

import ReasoningCard from "./ReasoningCard";
import type { PermissionDecision } from "../../state/agent";

interface PermissionCardProps {
  stepId: string;
  timestamp: number;
  capability: string;
  decision?: PermissionDecision;
  onDecide: (decision: PermissionDecision) => void;
}

export default function PermissionCard({
  stepId,
  timestamp,
  capability,
  decision,
  onDecide,
}: PermissionCardProps) {
  return (
    <ReasoningCard stepId={stepId} timestamp={timestamp} variant="permission">
      <p className="text-sm text-gray-300 mb-2">
        Requesting capability: <code className="text-purple-400">{capability}</code>
      </p>

      {decision ? (
        <span className={`text-xs px-2 py-1 rounded ${
          decision === "Deny" ? "bg-red-900/50 text-red-400" : "bg-green-900/50 text-green-400"
        }`}>
          {decision}
        </span>
      ) : (
        <div className="flex gap-2">
          <button
            onClick={() => onDecide("Deny")}
            className="px-3 py-1 text-xs rounded bg-red-800 hover:bg-red-700 text-white transition-colors"
          >
            Deny
          </button>
          <button
            onClick={() => onDecide("AllowOnce")}
            className="px-3 py-1 text-xs rounded bg-amber-700 hover:bg-amber-600 text-white transition-colors"
          >
            Allow once
          </button>
          <button
            onClick={() => onDecide("AllowAlways")}
            className="px-3 py-1 text-xs rounded bg-green-700 hover:bg-green-600 text-white transition-colors"
          >
            Allow always
          </button>
        </div>
      )}
    </ReasoningCard>
  );
}
