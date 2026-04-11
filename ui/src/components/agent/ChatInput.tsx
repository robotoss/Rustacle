/**
 * ChatInput — multiline text input for sending messages to the agent.
 *
 * Enter sends, Shift+Enter inserts newline. Disabled during active turn.
 */

import { useCallback, useRef, useState } from "react";

interface ChatInputProps {
  disabled: boolean;
  activeModel: string | null;
  onSend: (message: string) => void;
  onStop: () => void;
  isRunning: boolean;
}

export default function ChatInput({ disabled, activeModel, onSend, onStop, isRunning }: ChatInputProps) {
  const [text, setText] = useState("");
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
      if (e.key === "Enter" && !e.shiftKey) {
        e.preventDefault();
        const trimmed = text.trim();
        if (trimmed && !disabled) {
          onSend(trimmed);
          setText("");
          if (textareaRef.current) {
            textareaRef.current.style.height = "auto";
          }
        }
      }
    },
    [text, disabled, onSend]
  );

  const handleInput = useCallback((e: React.ChangeEvent<HTMLTextAreaElement>) => {
    setText(e.target.value);
    // Auto-resize
    const el = e.target;
    el.style.height = "auto";
    el.style.height = `${Math.min(el.scrollHeight, 160)}px`;
  }, []);

  return (
    <div className="border-t border-gray-700 px-3 py-2">
      <div className="text-xs mb-1 truncate">
        {activeModel ? (
          <span className="text-gray-600">Model: {activeModel}</span>
        ) : (
          <span className="text-amber-500">No model selected — configure in Settings &rarr; Model Profiles</span>
        )}
      </div>
      <div className="flex items-end gap-2">
        <textarea
          ref={textareaRef}
          value={text}
          onChange={handleInput}
          onKeyDown={handleKeyDown}
          disabled={disabled}
          placeholder={disabled ? "Agent is thinking..." : "Message the agent... (Enter to send)"}
          rows={1}
          className="flex-1 resize-none bg-gray-800 text-gray-200 text-sm rounded px-3 py-2 border border-gray-600 focus:border-blue-500 focus:outline-none placeholder:text-gray-600 disabled:opacity-50 disabled:cursor-not-allowed"
          style={{ maxHeight: "160px" }}
        />
        {isRunning ? (
          <button
            onClick={onStop}
            className="px-3 py-2 text-xs rounded bg-red-800 hover:bg-red-700 text-white transition-colors shrink-0"
          >
            Stop
          </button>
        ) : (
          <button
            onClick={() => {
              const trimmed = text.trim();
              if (trimmed) {
                onSend(trimmed);
                setText("");
              }
            }}
            disabled={disabled || !text.trim()}
            className="px-3 py-2 text-xs rounded bg-blue-700 hover:bg-blue-600 text-white transition-colors shrink-0 disabled:opacity-50 disabled:cursor-not-allowed"
          >
            Send
          </button>
        )}
      </div>
    </div>
  );
}
