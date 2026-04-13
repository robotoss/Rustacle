import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { PaletteItem, RankedEntry } from "./PaletteEntry";
import { rankEntries } from "./fuzzySearch";

const RECENT_KEY = "rustacle:palette:recent";
const MAX_RECENT = 20;
const MAX_VISIBLE = 50;

interface CommandPaletteProps {
  open: boolean;
  onClose: () => void;
  entries: PaletteItem[];
}

export default function CommandPalette({ open, onClose, entries }: CommandPaletteProps) {
  const [query, setQuery] = useState("");
  const [selectedIdx, setSelectedIdx] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);

  // Load recent IDs from localStorage.
  const recentIds = useMemo(() => {
    try {
      return JSON.parse(localStorage.getItem(RECENT_KEY) ?? "[]") as string[];
    } catch {
      return [];
    }
  }, [open]); // eslint-disable-line react-hooks/exhaustive-deps

  const ranked = useMemo(
    () => rankEntries(query, entries, recentIds).slice(0, MAX_VISIBLE),
    [query, entries, recentIds]
  );

  // Focus input on open.
  useEffect(() => {
    if (open) {
      setQuery("");
      setSelectedIdx(0);
      setTimeout(() => inputRef.current?.focus(), 0);
    }
  }, [open]);

  // Reset selection when results change.
  useEffect(() => {
    setSelectedIdx(0);
  }, [ranked.length]);

  const execute = useCallback(
    (entry: RankedEntry) => {
      // Update recents.
      const updated = [entry.item.id, ...recentIds.filter((id) => id !== entry.item.id)].slice(
        0,
        MAX_RECENT
      );
      localStorage.setItem(RECENT_KEY, JSON.stringify(updated));

      onClose();
      entry.item.action();
    },
    [onClose, recentIds]
  );

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      switch (e.key) {
        case "ArrowDown":
          e.preventDefault();
          setSelectedIdx((i) => Math.min(i + 1, ranked.length - 1));
          break;
        case "ArrowUp":
          e.preventDefault();
          setSelectedIdx((i) => Math.max(i - 1, 0));
          break;
        case "Enter":
          e.preventDefault();
          if (ranked[selectedIdx]) execute(ranked[selectedIdx]);
          break;
        case "Escape":
          e.preventDefault();
          onClose();
          break;
      }
    },
    [ranked, selectedIdx, execute, onClose]
  );

  if (!open) return null;

  return (
    <div
      className="fixed inset-0 z-50 flex items-start justify-center pt-[15vh] bg-black/50"
      onClick={onClose}
      role="dialog"
      aria-modal="true"
      aria-label="Command Palette"
    >
      <div
        className="w-full max-w-lg bg-gray-900 border border-gray-700 rounded-lg shadow-2xl overflow-hidden"
        onClick={(e) => e.stopPropagation()}
        onKeyDown={handleKeyDown}
      >
        <input
          ref={inputRef}
          type="text"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="Type a command..."
          className="w-full px-4 py-3 bg-transparent text-white text-sm border-b border-gray-700 outline-none placeholder-gray-500"
          aria-label="Search commands"
        />

        <ul className="max-h-72 overflow-y-auto" role="listbox">
          {ranked.map((entry, idx) => (
            <li
              key={entry.item.id}
              role="option"
              aria-selected={idx === selectedIdx}
              onClick={() => execute(entry)}
              className={`flex items-center justify-between px-4 py-2 text-sm cursor-pointer transition-colors ${
                idx === selectedIdx
                  ? "bg-indigo-700 text-white"
                  : "text-gray-300 hover:bg-gray-800"
              }`}
            >
              <span>
                <HighlightedLabel label={entry.item.label} highlights={entry.highlights} />
                <span className="ml-2 text-xs text-gray-500">{entry.item.category}</span>
              </span>
              {entry.item.shortcut && (
                <kbd className="text-xs text-gray-400 bg-gray-800 px-1.5 py-0.5 rounded">
                  {entry.item.shortcut}
                </kbd>
              )}
            </li>
          ))}

          {ranked.length === 0 && (
            <li className="px-4 py-6 text-sm text-gray-500 text-center">No matching commands</li>
          )}
        </ul>
      </div>
    </div>
  );
}

function HighlightedLabel({
  label,
  highlights,
}: {
  label: string;
  highlights: [number, number][];
}) {
  if (highlights.length === 0) return <>{label}</>;

  const parts: React.ReactNode[] = [];
  let cursor = 0;

  for (const [start, end] of highlights) {
    if (cursor < start) {
      parts.push(label.slice(cursor, start));
    }
    parts.push(
      <span key={start} className="text-yellow-300 font-semibold">
        {label.slice(start, end)}
      </span>
    );
    cursor = end;
  }

  if (cursor < label.length) {
    parts.push(label.slice(cursor));
  }

  return <>{parts}</>;
}
