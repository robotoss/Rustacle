/**
 * AgentPanel — collapsible side panel that streams reasoning steps as typed cards.
 *
 * Toggle with Ctrl/Cmd+J. Steps stream in real time via Tauri events.
 * Includes chat input, mode selector, and profile switcher.
 */

import { useCallback, useEffect, useReducer, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { commands } from "../../../bindings";
import { agentReducer, initialAgentState } from "../../state/agent";
import type { ReasoningStep, PermissionDecision, AgentMode } from "../../state/agent";
import ThoughtCard from "./ThoughtCard";
import ToolCallCard from "./ToolCallCard";
import PermissionCard from "./PermissionCard";
import CostBadge from "./CostBadge";
import ChatInput from "./ChatInput";
import ModeSelector from "./ModeSelector";
import ProfileSwitcher from "./ProfileSwitcher";

/** Map IPC event payload to local ReasoningStep shape. */
function mapEventToStep(payload: {
  id: string;
  parent_id?: string;
  turn_id: string;
  ts_ms: number;
  step: { type: string; data: Record<string, unknown> };
}): ReasoningStep {
  const { id, parent_id, turn_id, ts_ms, step: raw } = payload;

  let step: ReasoningStep["step"];
  switch (raw.type) {
    case "Thought":
      step = { kind: "Thought", data: { text: raw.data.text as string, partial: raw.data.partial as boolean } };
      break;
    case "ToolCall":
      step = { kind: "ToolCall", data: { tool: raw.data.tool as string, args: raw.data.args, tab_target: raw.data.tab_target as number | undefined } };
      break;
    case "ToolResult":
      step = { kind: "ToolResult", data: { tool: raw.data.tool as string, ok: raw.data.ok as boolean, summary: raw.data.summary as string, duration_ms: raw.data.duration_ms as number } };
      break;
    case "PermissionAsk":
      step = { kind: "PermissionAsk", data: { capability: raw.data.capability as string, decision: raw.data.decision as PermissionDecision | undefined } };
      break;
    case "Answer":
      step = { kind: "Answer", data: { text: raw.data.text as string } };
      break;
    case "Error":
      step = { kind: "Error", data: { message: raw.data.message as string, retryable: raw.data.retryable as boolean } };
      break;
    default:
      step = { kind: "Error", data: { message: `Unknown step type: ${raw.type}`, retryable: false } };
  }

  return { id, parent_id, turn_id, ts_ms, step };
}

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
  }, [state.turns]);

  // Wire up Tauri event listeners
  useEffect(() => {
    const unlisteners: Promise<() => void>[] = [];

    unlisteners.push(
      listen("agent:reasoning", (event) => {
        console.log("[agent:reasoning] raw payload:", JSON.stringify(event.payload).slice(0, 500));
        try {
          const step = mapEventToStep(event.payload as Parameters<typeof mapEventToStep>[0]);
          console.log("[agent:reasoning] mapped step:", step.id, step.step.kind, "turn:", step.turn_id);
          dispatch({ type: "ADD_STEP", step });
        } catch (e) {
          console.error("[agent:reasoning] mapEventToStep FAILED:", e, "payload:", event.payload);
        }
      })
    );

    unlisteners.push(
      listen("agent:cost", (event) => {
        console.log("[agent:cost]", event.payload);
        const payload = event.payload as { turn_id: string; input_tokens: number; output_tokens: number };
        dispatch({ type: "UPDATE_COST", cost: payload });
      })
    );

    unlisteners.push(
      listen("agent:turn_end", (event) => {
        console.log("[agent:turn_end]", event.payload);
        const payload = event.payload as {
          turn_id: string;
          duration_ms: number;
          input_tokens: number;
          output_tokens: number;
        };
        dispatch({
          type: "FINISH_TURN",
          turnId: payload.turn_id,
          duration_ms: payload.duration_ms,
          input_tokens: payload.input_tokens,
          output_tokens: payload.output_tokens,
        });
      })
    );

    return () => {
      for (const p of unlisteners) {
        p.then((f) => f());
      }
    };
  }, []);

  // Auto-select first profile when none selected
  useEffect(() => {
    if (!state.currentProfile) {
      commands.listModelProfiles().then((res) => {
        if (res.status === "ok" && res.data.profiles.length > 0) {
          dispatch({ type: "SET_PROFILE", profile: res.data.profiles[0].name });
        }
      }).catch(() => {});
    }
  }, [state.currentProfile]);

  // Reload profile on settings change
  useEffect(() => {
    const unlisten = listen("settings:changed", (event) => {
      const payload = event.payload as { key?: string };
      if (!payload.key || payload.key === "model.profiles") {
        // Re-check if current profile still exists, auto-select first if not
        commands.listModelProfiles().then((res) => {
          if (res.status === "ok") {
            const names = res.data.profiles.map((p) => p.name);
            if (state.currentProfile && !names.includes(state.currentProfile)) {
              dispatch({ type: "SET_PROFILE", profile: names[0] ?? null as unknown as string });
            } else if (!state.currentProfile && names.length > 0) {
              dispatch({ type: "SET_PROFILE", profile: names[0] });
            }
          }
        }).catch(() => {});
      }
    });
    return () => { unlisten.then((f) => f()); };
  }, [state.currentProfile]);

  const handleSend = useCallback(
    async (message: string) => {
      const model = state.currentProfile ?? "default";
      const mode = state.mode;

      try {
        console.log("[handleSend] sending:", message, "profile:", state.currentProfile, "mode:", mode);
        const result = await commands.sendPrompt({
          message,
          model_profile: state.currentProfile,
          mode: mode as "Chat" | "Plan" | "Ask",
        });
        console.log("[handleSend] result:", JSON.stringify(result));

        if (result.status === "ok") {
          console.log("[handleSend] START_TURN with turn_id:", result.data.turn_id);
          dispatch({
            type: "START_TURN",
            turnId: result.data.turn_id,
            userMessage: message,
            mode,
            model,
          });
        } else {
          console.error("[handleSend] sendPrompt failed:", result);
        }
      } catch {
        // Failed to send — don't lock the input
      }
    },
    [state.currentProfile, state.mode]
  );

  const handleStop = useCallback(async () => {
    if (state.activeTurnId) {
      try {
        await commands.stopTurn({ turn_id: state.activeTurnId });
      } catch {
        // Best effort
      }
      // Force end turn in UI immediately — don't wait for backend event
      dispatch({ type: "END_TURN" });
    }
  }, [state.activeTurnId]);

  const handlePermissionDecide = useCallback(
    async (stepId: string, decision: PermissionDecision) => {
      if (state.activeTurnId) {
        try {
          await commands.respondPermission({
            turn_id: state.activeTurnId,
            step_id: stepId,
            decision,
          });
        } catch {
          // Best effort
        }
      }
    },
    [state.activeTurnId]
  );

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
      <div className="flex items-center justify-between px-3 py-2 border-b border-gray-700 gap-2">
        <div className="flex items-center gap-2 min-w-0">
          <h2 className="text-sm font-semibold text-gray-300 shrink-0">Agent</h2>
          <ModeSelector
            mode={state.mode}
            onChange={(mode: AgentMode) => dispatch({ type: "SET_MODE", mode })}
            disabled={state.inputDisabled}
          />
          <ProfileSwitcher
            currentProfile={state.currentProfile}
            onChange={(profile: string) => dispatch({ type: "SET_PROFILE", profile })}
          />
        </div>
        <button
          onClick={() => dispatch({ type: "SET_PANEL", open: false })}
          className="text-gray-500 hover:text-white text-lg leading-none shrink-0"
          title="Close (Ctrl+J)"
        >
          &times;
        </button>
      </div>

      {/* Cost badge */}
      <CostBadge cost={state.cost} active={state.activeTurnId !== null} />

      {/* Debug: state summary */}
      <div className="text-xs text-gray-700 px-3 py-1 bg-gray-950 font-mono">
        turns={state.turns.length} active={state.activeTurnId ?? "null"} disabled={String(state.inputDisabled)}
        {state.turns.map((t) => ` | ${t.turnId.slice(-6)}: steps=${t.steps.length}`).join("")}
      </div>

      {/* Conversation history */}
      <div
        ref={scrollRef}
        className="flex-1 overflow-y-auto px-2 py-2 space-y-2"
        aria-live="polite"
        aria-atomic="false"
      >
        {state.turns.length === 0 && (
          <p className="text-sm text-gray-600 text-center mt-8">
            No conversation yet. Type a message below to start.
          </p>
        )}

        {state.turns.map((turn) => (
          <div key={turn.turnId} className="space-y-1">
            {/* User message bubble */}
            <div className="flex justify-end">
              <div className="max-w-[85%] bg-blue-900/40 border border-blue-800/50 rounded-lg px-3 py-2">
                <div className="flex items-center gap-2 mb-1">
                  <span className="text-xs text-blue-400 uppercase tracking-wide">{turn.mode}</span>
                  {turn.tokenUsage && (
                    <span className="text-xs text-gray-600">
                      {turn.tokenUsage.input + turn.tokenUsage.output} tok
                    </span>
                  )}
                </div>
                <p className="text-sm text-gray-200 whitespace-pre-wrap">{turn.userMessage}</p>
              </div>
            </div>

            {/* Reasoning steps */}
            {turn.steps.map((step) => renderStep(step, handlePermissionDecide))}

            {/* Turn separator */}
            {turn.endedAt && (
              <div className="flex items-center gap-2 py-1">
                <div className="flex-1 border-t border-gray-800" />
                <span className="text-xs text-gray-700">
                  {new Date(turn.endedAt).toLocaleTimeString()}
                </span>
                <div className="flex-1 border-t border-gray-800" />
              </div>
            )}
          </div>
        ))}
      </div>

      {/* Chat input */}
      <ChatInput
        disabled={state.inputDisabled}
        activeModel={state.currentProfile}
        onSend={handleSend}
        onStop={handleStop}
        isRunning={state.activeTurnId !== null}
      />
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
