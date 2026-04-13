import { useCallback, useRef, useState } from "react";
import type { SplitLayout } from "../../state/terminal";
import TerminalPane from "./TerminalPane";

interface SplitTreeProps {
  layout: SplitLayout | null;
  activeTabId: string | null;
  onFocusTab: (tabId: string) => void;
  onTitle: (tabId: string, title: string) => void;
  onResizeSplit: (nodeId: string, ratio: number) => void;
}

/**
 * Recursively renders a split layout tree. Leaves become TerminalPane
 * components; splits become flex containers with draggable dividers.
 */
export default function SplitTree({
  layout,
  activeTabId,
  onFocusTab,
  onTitle,
  onResizeSplit,
}: SplitTreeProps) {
  if (!layout) {
    return (
      <div className="flex items-center justify-center h-full text-gray-500 text-sm">
        No terminal open. Press Ctrl+T to create one.
      </div>
    );
  }

  return (
    <div className="h-full w-full">
      <SplitNode
        node={layout}
        activeTabId={activeTabId}
        onFocusTab={onFocusTab}
        onTitle={onTitle}
        onResizeSplit={onResizeSplit}
      />
    </div>
  );
}

interface SplitNodeProps {
  node: SplitLayout;
  activeTabId: string | null;
  onFocusTab: (tabId: string) => void;
  onTitle: (tabId: string, title: string) => void;
  onResizeSplit: (nodeId: string, ratio: number) => void;
}

function SplitNode({ node, activeTabId, onFocusTab, onTitle, onResizeSplit }: SplitNodeProps) {
  if (node.kind === "leaf") {
    return (
      <div
        className={`h-full w-full ${
          node.tab_id === activeTabId ? "ring-1 ring-indigo-500/30" : ""
        }`}
      >
        <TerminalPane
          tabId={node.tab_id}
          onFocus={() => onFocusTab(node.tab_id)}
          onTitle={(title) => onTitle(node.tab_id, title)}
        />
      </div>
    );
  }

  const isHorizontal = node.direction === "horizontal";

  return (
    <div
      className="h-full w-full flex"
      style={{ flexDirection: isHorizontal ? "row" : "column" }}
    >
      <div style={{ flex: `${node.ratio} 1 0%`, minWidth: 60, minHeight: 40, overflow: "hidden" }}>
        <SplitNode
          node={node.children[0]}
          activeTabId={activeTabId}
          onFocusTab={onFocusTab}
          onTitle={onTitle}
          onResizeSplit={onResizeSplit}
        />
      </div>

      <SplitDivider
        nodeId={node.id}
        direction={isHorizontal ? "horizontal" : "vertical"}
        onResize={(ratio) => onResizeSplit(node.id, ratio)}
      />

      {node.children.length > 1 && (
        <div
          style={{
            flex: `${1 - node.ratio} 1 0%`,
            minWidth: 60,
            minHeight: 40,
            overflow: "hidden",
          }}
        >
          <SplitNode
            node={node.children[1]}
            activeTabId={activeTabId}
            onFocusTab={onFocusTab}
            onTitle={onTitle}
            onResizeSplit={onResizeSplit}
          />
        </div>
      )}
    </div>
  );
}

interface SplitDividerProps {
  nodeId: string;
  direction: "horizontal" | "vertical";
  onResize: (ratio: number) => void;
}

function SplitDivider({ direction, onResize }: SplitDividerProps) {
  const dividerRef = useRef<HTMLDivElement>(null);
  const [dragging, setDragging] = useState(false);

  const handleMouseDown = useCallback(
    (e: React.MouseEvent) => {
      e.preventDefault();
      setDragging(true);

      const parent = dividerRef.current?.parentElement;
      if (!parent) return;

      const handleMouseMove = (ev: MouseEvent) => {
        const rect = parent.getBoundingClientRect();
        let ratio: number;
        if (direction === "horizontal") {
          ratio = (ev.clientX - rect.left) / rect.width;
        } else {
          ratio = (ev.clientY - rect.top) / rect.height;
        }
        ratio = Math.max(0.05, Math.min(0.95, ratio));
        onResize(ratio);
      };

      const handleMouseUp = () => {
        setDragging(false);
        document.removeEventListener("mousemove", handleMouseMove);
        document.removeEventListener("mouseup", handleMouseUp);
      };

      document.addEventListener("mousemove", handleMouseMove);
      document.addEventListener("mouseup", handleMouseUp);
    },
    [direction, onResize]
  );

  const isHorizontal = direction === "horizontal";

  return (
    <div
      ref={dividerRef}
      onMouseDown={handleMouseDown}
      className={`flex-shrink-0 bg-gray-700 hover:bg-indigo-500 transition-colors ${
        dragging ? "bg-indigo-500" : ""
      } ${isHorizontal ? "w-1 cursor-col-resize" : "h-1 cursor-row-resize"}`}
    />
  );
}
