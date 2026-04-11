/**
 * Base card wrapper for all reasoning step types.
 * Shared styling, step ID, and timestamp display.
 */

import type { ReactNode } from "react";

interface ReasoningCardProps {
  stepId: string;
  timestamp: number;
  variant: "thought" | "tool-call" | "tool-result" | "permission" | "answer" | "error";
  children: ReactNode;
}

const variantStyles: Record<string, string> = {
  thought: "border-l-blue-500",
  "tool-call": "border-l-amber-500",
  "tool-result": "border-l-green-500",
  permission: "border-l-purple-500",
  answer: "border-l-emerald-500",
  error: "border-l-red-500",
};

function formatTime(ms: number): string {
  const d = new Date(ms);
  return d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", second: "2-digit" });
}

export default function ReasoningCard({ stepId, timestamp, variant, children }: ReasoningCardProps) {
  return (
    <div
      className={`border-l-2 ${variantStyles[variant] ?? "border-l-gray-500"} bg-gray-800/50 rounded-r px-3 py-2 mb-1`}
      data-step-id={stepId}
      role="article"
      aria-label={`${variant} step`}
    >
      <div className="flex items-center justify-between mb-1">
        <span className="text-xs text-gray-500 uppercase tracking-wide">{variant}</span>
        <span className="text-xs text-gray-600">{formatTime(timestamp)}</span>
      </div>
      {children}
    </div>
  );
}
