/**
 * ThoughtCard — streams partial text from the LLM.
 * Partial thoughts show a blinking cursor; complete thoughts don't.
 */

import ReasoningCard from "./ReasoningCard";

interface ThoughtCardProps {
  stepId: string;
  timestamp: number;
  text: string;
  partial: boolean;
}

export default function ThoughtCard({ stepId, timestamp, text, partial }: ThoughtCardProps) {
  return (
    <ReasoningCard stepId={stepId} timestamp={timestamp} variant="thought">
      <p className="text-sm text-gray-300 whitespace-pre-wrap">
        {text}
        {partial && <span className="inline-block w-1.5 h-4 bg-blue-400 animate-pulse ml-0.5 align-text-bottom" />}
      </p>
    </ReasoningCard>
  );
}
