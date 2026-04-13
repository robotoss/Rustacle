import { useCallback, useRef } from "react";
import type { TabInfo } from "../../state/terminal";

interface TabBarProps {
  tabs: TabInfo[];
  activeTabId: string | null;
  onSelect: (tabId: string) => void;
  onClose: (tabId: string) => void;
  onNew: () => void;
  onReorder: (tabId: string, newIndex: number) => void;
  /** Called when a tool-call card is dropped on a tab (Phase 3). */
  onToolDrop?: (tabId: string, stepId: string) => void;
}

export default function TabBar({
  tabs,
  activeTabId,
  onSelect,
  onClose,
  onNew,
  onReorder,
  onToolDrop,
}: TabBarProps) {
  const dragIdRef = useRef<string | null>(null);

  const handleDragStart = useCallback(
    (e: React.DragEvent, tabId: string) => {
      dragIdRef.current = tabId;
      e.dataTransfer.setData("text/tab-id", tabId);
      e.dataTransfer.effectAllowed = "move";
    },
    []
  );

  const handleDragOver = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    e.dataTransfer.dropEffect = "move";
  }, []);

  const handleDrop = useCallback(
    (e: React.DragEvent, targetIndex: number, targetTabId: string) => {
      e.preventDefault();

      // Handle tool-call card drops (Phase 3).
      const stepId = e.dataTransfer.getData("text/step-id");
      if (stepId && onToolDrop) {
        onToolDrop(targetTabId, stepId);
        return;
      }

      // Handle tab reorder.
      const draggedId = e.dataTransfer.getData("text/tab-id");
      if (draggedId && draggedId !== targetTabId) {
        onReorder(draggedId, targetIndex);
      }
      dragIdRef.current = null;
    },
    [onReorder, onToolDrop]
  );

  const handleMiddleClick = useCallback(
    (e: React.MouseEvent, tabId: string) => {
      if (e.button === 1) {
        e.preventDefault();
        onClose(tabId);
      }
    },
    [onClose]
  );

  return (
    <div className="flex items-center bg-gray-900 border-b border-gray-700 px-1 h-8 select-none">
      {tabs.map((tab, idx) => (
        <div
          key={tab.id}
          draggable
          onDragStart={(e) => handleDragStart(e, tab.id)}
          onDragOver={handleDragOver}
          onDrop={(e) => handleDrop(e, idx, tab.id)}
          onClick={() => onSelect(tab.id)}
          onMouseDown={(e) => handleMiddleClick(e, tab.id)}
          className={`flex items-center gap-1.5 px-3 py-1 text-xs cursor-pointer border-r border-gray-700 transition-colors ${
            tab.id === activeTabId
              ? "bg-gray-800 text-white"
              : "text-gray-400 hover:text-gray-200 hover:bg-gray-800/50"
          }`}
        >
          <span className={`text-[10px] ${tab.alive ? "text-green-400" : "text-red-400"}`}>
            {tab.alive ? "\u25CF" : "\u25CB"}
          </span>
          <span className="text-[10px] text-gray-500">{idx + 1}</span>
          <span className="truncate max-w-[120px]">{tab.title}</span>
          <button
            onClick={(e) => {
              e.stopPropagation();
              onClose(tab.id);
            }}
            className="ml-1 text-gray-500 hover:text-red-400 transition-colors"
            aria-label={`Close ${tab.title}`}
          >
            \u00D7
          </button>
        </div>
      ))}

      <button
        onClick={onNew}
        className="px-2 py-1 text-xs text-gray-500 hover:text-white transition-colors"
        aria-label="New tab"
        title="New Tab (Ctrl+T)"
      >
        +
      </button>
    </div>
  );
}
