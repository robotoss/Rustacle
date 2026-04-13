/**
 * Agent state management for reasoning steps, conversation turns, and cost tracking.
 *
 * Uses a reducer pattern with conversation history support.
 */

export type AgentMode = "Chat" | "Plan" | "Ask";

export type StepKind =
  | { kind: "Thought"; data: { text: string; partial: boolean } }
  | { kind: "ToolCall"; data: { tool: string; args: unknown; tab_target?: number } }
  | { kind: "ToolResult"; data: { tool: string; ok: boolean; summary: string; duration_ms: number } }
  | { kind: "PermissionAsk"; data: { capability: string; decision?: PermissionDecision } }
  | { kind: "Answer"; data: { text: string } }
  | { kind: "Error"; data: { message: string; retryable: boolean } };

export type PermissionDecision = "Deny" | "AllowOnce" | "AllowAlways";

export interface ReasoningStep {
  id: string;
  parent_id?: string;
  turn_id: string;
  ts_ms: number;
  step: StepKind;
}

export interface CostSample {
  turn_id: string;
  input_tokens: number;
  output_tokens: number;
}

export interface ConversationTurn {
  turnId: string;
  userMessage: string;
  mode: AgentMode;
  model: string;
  steps: ReasoningStep[];
  startedAt: number;
  endedAt?: number;
  tokenUsage?: { input: number; output: number };
}

export interface AgentState {
  turns: ConversationTurn[];
  cost: CostSample;
  activeTurnId: string | null;
  panelOpen: boolean;
  mode: AgentMode;
  currentProfile: string | null;
  inputDisabled: boolean;
}

export const initialAgentState: AgentState = {
  turns: [],
  cost: { turn_id: "", input_tokens: 0, output_tokens: 0 },
  activeTurnId: null,
  panelOpen: false,
  mode: "Chat",
  currentProfile: null,
  inputDisabled: false,
};

export type AgentAction =
  | { type: "ADD_STEP"; step: ReasoningStep }
  | { type: "UPDATE_PARTIAL_THOUGHT"; stepId: string; text: string }
  | { type: "UPDATE_COST"; cost: CostSample }
  | { type: "START_TURN"; turnId: string; userMessage: string; mode: AgentMode; model: string }
  | { type: "END_TURN" }
  | { type: "FINISH_TURN"; turnId: string; duration_ms: number; input_tokens: number; output_tokens: number }
  | { type: "TOGGLE_PANEL" }
  | { type: "SET_PANEL"; open: boolean }
  | { type: "SET_MODE"; mode: AgentMode }
  | { type: "SET_PROFILE"; profile: string }
  | { type: "REPLACE_TURN_ID"; oldId: string; newId: string }
  | { type: "CLEAR_CONVERSATION" }
  | { type: "REROUTE_TOOL_CALL"; stepId: string; newTabTarget: number };

export function agentReducer(state: AgentState, action: AgentAction): AgentState {
  switch (action.type) {
    case "ADD_STEP": {
      const turnIdx = state.turns.findIndex((t) => t.turnId === action.step.turn_id);

      if (turnIdx !== -1) {
        // Turn exists — check if step already exists (streaming update by same ID).
        const turn = state.turns[turnIdx];
        const stepIdx = turn.steps.findIndex((s) => s.id === action.step.id);

        let newSteps: ReasoningStep[];
        if (stepIdx !== -1) {
          // UPDATE existing step in place (streaming: same thought ID, new text).
          newSteps = [...turn.steps];
          newSteps[stepIdx] = action.step;
        } else {
          // APPEND new step.
          newSteps = [...turn.steps, action.step];
        }

        const turns = state.turns.map((t, i) =>
          i === turnIdx ? { ...t, steps: newSteps } : t
        );
        return { ...state, turns };
      }

      // Turn doesn't exist yet (event arrived before START_TURN) — create it.
      const newTurn: ConversationTurn = {
        turnId: action.step.turn_id,
        userMessage: "",
        mode: state.mode,
        model: state.currentProfile ?? "default",
        steps: [action.step],
        startedAt: Date.now(),
      };
      return {
        ...state,
        activeTurnId: action.step.turn_id,
        inputDisabled: true,
        turns: [...state.turns, newTurn],
      };
    }

    case "UPDATE_PARTIAL_THOUGHT": {
      const turns = state.turns.map((t) => {
        if (t.turnId !== state.activeTurnId) return t;
        const steps = t.steps.map((s) =>
          s.id === action.stepId && s.step.kind === "Thought"
            ? { ...s, step: { ...s.step, data: { ...s.step.data, text: action.text } } as StepKind }
            : s
        );
        return { ...t, steps };
      });
      return { ...state, turns };
    }

    case "UPDATE_COST":
      return { ...state, cost: action.cost };

    case "START_TURN": {
      // If turn already exists (created early by ADD_STEP), patch in user message.
      const existing = state.turns.find((t) => t.turnId === action.turnId);
      if (existing) {
        const turns = state.turns.map((t) =>
          t.turnId === action.turnId
            ? { ...t, userMessage: action.userMessage, mode: action.mode, model: action.model }
            : t
        );
        return { ...state, activeTurnId: action.turnId, inputDisabled: true, turns };
      }
      const newTurn: ConversationTurn = {
        turnId: action.turnId,
        userMessage: action.userMessage,
        mode: action.mode,
        model: action.model,
        steps: [],
        startedAt: Date.now(),
      };
      return {
        ...state,
        activeTurnId: action.turnId,
        inputDisabled: true,
        turns: [...state.turns, newTurn],
      };
    }

    case "END_TURN":
      return { ...state, activeTurnId: null, inputDisabled: false };

    case "FINISH_TURN": {
      const turns = state.turns.map((t) =>
        t.turnId === action.turnId
          ? {
              ...t,
              endedAt: Date.now(),
              tokenUsage: { input: action.input_tokens, output: action.output_tokens },
            }
          : t
      );
      return { ...state, turns, activeTurnId: null, inputDisabled: false };
    }

    case "TOGGLE_PANEL":
      return { ...state, panelOpen: !state.panelOpen };

    case "SET_PANEL":
      return { ...state, panelOpen: action.open };

    case "SET_MODE":
      return { ...state, mode: action.mode };

    case "SET_PROFILE":
      return { ...state, currentProfile: action.profile };

    case "REPLACE_TURN_ID": {
      const turns = state.turns.map((t) =>
        t.turnId === action.oldId ? { ...t, turnId: action.newId } : t
      );
      const activeTurnId = state.activeTurnId === action.oldId ? action.newId : state.activeTurnId;
      return { ...state, turns, activeTurnId };
    }

    case "CLEAR_CONVERSATION":
      return { ...state, turns: [], cost: { turn_id: "", input_tokens: 0, output_tokens: 0 } };

    case "REROUTE_TOOL_CALL": {
      const turns = state.turns.map((t) => {
        if (t.turnId !== state.activeTurnId) return t;
        const steps = t.steps.map((s) =>
          s.id === action.stepId && s.step.kind === "ToolCall"
            ? { ...s, step: { ...s.step, data: { ...s.step.data, tab_target: action.newTabTarget } } as StepKind }
            : s
        );
        return { ...t, steps };
      });
      return { ...state, turns };
    }
  }
}
