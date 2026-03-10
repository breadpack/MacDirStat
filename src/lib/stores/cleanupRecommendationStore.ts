import { writable } from "svelte/store";
import type { CleanupRecommendation, CleanupScanProgress } from "../types";

export const recommendations = writable<CleanupRecommendation[]>([]);
export const cleanupScanning = writable(false);
export const cleanupProgress = writable<CleanupScanProgress | null>(null);
export const selectedPatternIds = writable<Set<string>>(new Set());

export function resetCleanupRecommendations() {
  recommendations.set([]);
  cleanupScanning.set(false);
  cleanupProgress.set(null);
  selectedPatternIds.set(new Set());
}

export function removeRecommendation(patternId: string) {
  recommendations.update((recs) => recs.filter((r) => r.pattern_id !== patternId));
  selectedPatternIds.update((ids) => {
    ids.delete(patternId);
    return new Set(ids);
  });
}
