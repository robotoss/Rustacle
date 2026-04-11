/**
 * Agent state management for reasoning steps and cost tracking.
 *
 * Uses a simple reducer pattern. Can migrate to Zustand when
 * the settings/persistence layer lands in S5.
 */

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

export interface AgentState {
  steps: ReasoningStep[];
  cost: CostSample;
  activeTurnId: string | null;
  panelOpen: boolean;
}

export const initialAgentState: AgentState = {
  steps: [],
  cost: { turn_id: "", input_tokens: 0, output_tokens: 0 },
  activeTurnId: null,
  panelOpen: false,
};

export type AgentAction =
  | { type: "ADD_STEP"; step: ReasoningStep }
  | { type: "UPDATE_PARTIAL_THOUGHT"; stepId: string; text: string }
  | { type: "UPDATE_COST"; cost: CostSample }
  | { type: "START_TURN"; turnId: string }
  | { type: "END_TURN" }
  | { type: "TOGGLE_PANEL" }
  | { type: "SET_PANEL"; open: boolean }
  | { type: "CLEAR_STEPS" };

export function agentReducer(state: AgentState, action: AgentAction): AgentState {
  switch (action.type) {
    case "ADD_STEP":
      return { ...state, steps: [...state.steps, action.step] };

    case "UPDATE_PARTIAL_THOUGHT": {
      const steps = state.steps.map((s) =>
        s.id === action.stepId && s.step.kind === "Thought"
          ? { ...s, step: { ...s.step, data: { ...s.step.data, text: action.text } } as StepKind }
          : s
      );
      return { ...state, steps };
    }

    case "UPDATE_COST":
      return { ...state, cost: action.cost };

    case "START_TURN":
      return { ...state, activeTurnId: action.turnId, steps: [] };

    case "END_TURN":
      return { ...state, activeTurnId: null };

    case "TOGGLE_PANEL":
      return { ...state, panelOpen: !state.panelOpen };

    case "SET_PANEL":
      return { ...state, panelOpen: action.open };

    case "CLEAR_STEPS":
      return { ...state, steps: [] };
  }
}
