/**
 * AgentPanel — collapsible side panel that streams reasoning steps as typed cards.
 *
 * Toggle with Ctrl/Cmd+J. Steps stream in real time via Tauri events.
 */

import { useCallback, useEffect, useReducer, useRef } from "react";
import { agentReducer, initialAgentState } from "../../state/agent";
import type { ReasoningStep, PermissionDecision } from "../../state/agent";
import ThoughtCard from "./ThoughtCard";
import ToolCallCard from "./ToolCallCard";
import PermissionCard from "./PermissionCard";
import CostBadge from "./CostBadge";

export default function AgentPanel() {
  const [state, dispatch] = useReducer(agentReducer, initialAgentState);
  const scrollRef = useRef<HTMLDivElement>(null);

  // Keyboard shortcut: Ctrl/Cmd + J
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key === "j") {
        e.preventDefault();
        dispatch({ type: "TOGGLE_PANEL" });
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, []);

  // Auto-scroll to bottom when new steps arrive
  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [state.steps.length]);

  // TODO: Wire up Tauri event listener for agent.reasoning and agent.cost
  // events.listen<ReasoningStep>("agent:reasoning", (event) => {
  //   dispatch({ type: "ADD_STEP", step: event.payload });
  // });

  const handlePermissionDecide = useCallback(
    (_stepId: string, _decision: PermissionDecision) => {
      // TODO: Send permission decision back to Rust via Tauri command
    },
    []
  );

  const handleStop = useCallback(() => {
    // TODO: Send cancel command to Rust via Tauri command
  }, []);

  if (!state.panelOpen) {
    return (
      <button
        onClick={() => dispatch({ type: "SET_PANEL", open: true })}
        className="fixed right-4 top-14 z-50 px-2 py-1 text-xs bg-gray-800 text-gray-400 rounded hover:bg-gray-700 hover:text-white transition-colors"
        title="Open Agent Panel (Ctrl+J)"
      >
        Agent
      </button>
    );
  }

  return (
    <div
      className="fixed right-0 top-10 bottom-0 w-96 bg-gray-900 border-l border-gray-700 flex flex-col z-40"
      role="complementary"
      aria-label="Agent reasoning panel"
    >
      {/* Header */}
      <div className="flex items-center justify-between px-3 py-2 border-b border-gray-700">
        <h2 className="text-sm font-semibold text-gray-300">Agent</h2>
        <div className="flex items-center gap-2">
          {state.activeTurnId && (
            <button
              onClick={handleStop}
              className="px-2 py-0.5 text-xs rounded bg-red-800 hover:bg-red-700 text-white transition-colors"
            >
              Stop
            </button>
          )}
          <button
            onClick={() => dispatch({ type: "SET_PANEL", open: false })}
            className="text-gray-500 hover:text-white text-lg leading-none"
            title="Close (Ctrl+J)"
          >
            &times;
          </button>
        </div>
      </div>

      {/* Cost badge */}
      <CostBadge cost={state.cost} active={state.activeTurnId !== null} />

      {/* Steps */}
      <div
        ref={scrollRef}
        className="flex-1 overflow-y-auto px-2 py-2 space-y-1"
        aria-live="polite"
        aria-atomic="false"
      >
        {state.steps.length === 0 && (
          <p className="text-sm text-gray-600 text-center mt-8">
            No reasoning steps yet. Start a conversation to see agent thinking.
          </p>
        )}

        {state.steps.map((step) => renderStep(step, handlePermissionDecide))}
      </div>
    </div>
  );
}

function renderStep(
  step: ReasoningStep,
  onPermissionDecide: (stepId: string, decision: PermissionDecision) => void
) {
  const { id, ts_ms, step: kind } = step;

  switch (kind.kind) {
    case "Thought":
      return (
        <ThoughtCard
          key={id}
          stepId={id}
          timestamp={ts_ms}
          text={kind.data.text}
          partial={kind.data.partial}
        />
      );

    case "ToolCall":
      return (
        <ToolCallCard
          key={id}
          stepId={id}
          timestamp={ts_ms}
          tool={kind.data.tool}
          args={kind.data.args}
        />
      );

    case "ToolResult":
      return (
        <ToolCallCard
          key={id}
          stepId={id}
          timestamp={ts_ms}
          tool={kind.data.tool}
          args={{}}
          result={{
            ok: kind.data.ok,
            summary: kind.data.summary,
            duration_ms: kind.data.duration_ms,
          }}
        />
      );

    case "PermissionAsk":
      return (
        <PermissionCard
          key={id}
          stepId={id}
          timestamp={ts_ms}
          capability={kind.data.capability}
          decision={kind.data.decision}
          onDecide={(decision) => onPermissionDecide(id, decision)}
        />
      );

    case "Answer":
      return (
        <div key={id} className="border-l-2 border-l-emerald-500 bg-gray-800/50 rounded-r px-3 py-2 mb-1">
          <div className="text-xs text-gray-500 uppercase tracking-wide mb-1">answer</div>
          <div className="text-sm text-gray-200 whitespace-pre-wrap">{kind.data.text}</div>
        </div>
      );

    case "Error":
      return (
        <div key={id} className="border-l-2 border-l-red-500 bg-gray-800/50 rounded-r px-3 py-2 mb-1">
          <div className="flex items-center gap-2 mb-1">
            <span className="text-xs text-gray-500 uppercase tracking-wide">error</span>
            {kind.data.retryable && (
              <span className="text-xs bg-amber-900/50 text-amber-400 px-1 rounded">retryable</span>
            )}
          </div>
          <p className="text-sm text-red-400">{kind.data.message}</p>
        </div>
      );
  }
}
