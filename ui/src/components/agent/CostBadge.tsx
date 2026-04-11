/**
 * CostBadge — live token usage and cost display.
 */

import type { CostSample } from "../../state/agent";

interface CostBadgeProps {
  cost: CostSample;
  active: boolean;
}

export default function CostBadge({ cost, active }: CostBadgeProps) {
  const total = cost.input_tokens + cost.output_tokens;
  if (total === 0 && !active) return null;

  return (
    <div className="flex items-center gap-3 text-xs text-gray-500 px-3 py-1 bg-gray-800/50 rounded">
      {active && (
        <span className="flex items-center gap-1">
          <span className="w-1.5 h-1.5 rounded-full bg-green-500 animate-pulse" />
          running
        </span>
      )}
      <span>in: {cost.input_tokens.toLocaleString()}</span>
      <span>out: {cost.output_tokens.toLocaleString()}</span>
      <span className="text-gray-600">total: {total.toLocaleString()}</span>
    </div>
  );
}
