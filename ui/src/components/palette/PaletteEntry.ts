/** A single command palette entry. */
export interface PaletteItem {
  id: string;
  label: string;
  shortcut?: string;
  category: string;
  source: "builtin" | "plugin";
  action: () => void;
}

/** A palette entry ranked by fuzzy match. */
export interface RankedEntry {
  item: PaletteItem;
  score: number;
  /** Pairs of [start, end) for highlighted characters. */
  highlights: [number, number][];
}
