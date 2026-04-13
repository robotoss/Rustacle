import type { PaletteItem, RankedEntry } from "./PaletteEntry";

/**
 * Character-by-character fuzzy match with scoring.
 *
 * Returns null if no match; otherwise returns a score and highlight ranges.
 */
export function fuzzyMatch(
  query: string,
  label: string
): { score: number; highlights: [number, number][] } | null {
  if (query.length === 0) return { score: 0, highlights: [] };

  const lowerQuery = query.toLowerCase();
  const lowerLabel = label.toLowerCase();

  let qi = 0;
  let score = 0;
  let lastMatchIdx = -2;
  const highlights: [number, number][] = [];
  let runStart = -1;

  for (let li = 0; li < lowerLabel.length && qi < lowerQuery.length; li++) {
    if (lowerLabel[li] === lowerQuery[qi]) {
      // Consecutive bonus.
      if (li === lastMatchIdx + 1) {
        score += 3;
      } else {
        // Close previous highlight run.
        if (runStart >= 0) {
          highlights.push([runStart, lastMatchIdx + 1]);
        }
        runStart = li;
      }

      // Word-boundary bonus (first char or preceded by separator).
      if (li === 0 || /[\s\-_:./]/.test(label[li - 1])) {
        score += 5;
      }

      // Exact case bonus.
      if (label[li] === query[qi]) {
        score += 1;
      }

      score += 1; // Base match score.
      lastMatchIdx = li;
      qi++;
    }
  }

  if (qi < lowerQuery.length) return null; // Not all chars matched.

  // Close final highlight run.
  if (runStart >= 0) {
    highlights.push([runStart, lastMatchIdx + 1]);
  }

  // Prefix bonus: if the query matches from the start.
  if (highlights.length > 0 && highlights[0][0] === 0) {
    score += 10;
  }

  return { score, highlights };
}

/**
 * Rank palette entries by fuzzy match score with recency boost.
 */
export function rankEntries(
  query: string,
  entries: PaletteItem[],
  recentIds: string[]
): RankedEntry[] {
  const results: RankedEntry[] = [];

  for (const item of entries) {
    if (query.length === 0) {
      // Show all entries when no query, ranked by recency.
      const recencyIdx = recentIds.indexOf(item.id);
      const recencyBoost = recencyIdx >= 0 ? 100 - recencyIdx : 0;
      results.push({ item, score: recencyBoost, highlights: [] });
      continue;
    }

    const match = fuzzyMatch(query, item.label);
    if (!match) continue;

    // Recency boost: recently used entries score higher.
    const recencyIdx = recentIds.indexOf(item.id);
    const recencyBoost = recencyIdx >= 0 ? 20 - Math.min(recencyIdx, 20) : 0;

    results.push({
      item,
      score: match.score + recencyBoost,
      highlights: match.highlights,
    });
  }

  results.sort((a, b) => b.score - a.score);
  return results;
}
