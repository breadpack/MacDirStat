import { writable } from "svelte/store";
import type { CleanupAction } from "../types";
import { getCleanupActions, saveCleanupActions, executeCleanup } from "../api";

export const cleanupActions = writable<CleanupAction[]>([]);

let loaded = false;

export async function loadCleanupActions() {
  if (loaded) return;
  try {
    const actions = await getCleanupActions();
    cleanupActions.set(actions);
    loaded = true;
  } catch (e) {
    console.error("Failed to load cleanup actions:", e);
  }
}

export async function saveCleanups(actions: CleanupAction[]) {
  try {
    await saveCleanupActions(actions);
    cleanupActions.set(actions);
  } catch (e) {
    console.error("Failed to save cleanup actions:", e);
    throw e;
  }
}

export async function runCleanup(
  actionId: number,
  path: string,
  name: string,
): Promise<string> {
  return executeCleanup(actionId, path, name);
}

/** Force reload from disk (e.g. after settings change). */
export async function reloadCleanupActions() {
  loaded = false;
  await loadCleanupActions();
}
