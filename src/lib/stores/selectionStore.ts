import { writable } from "svelte/store";

export const selectedPath = writable<string | null>(null);
export const hoveredPath = writable<string | null>(null);
export const zoomRoot = writable<string | null>(null);

/** Toggle display of free disk space in treemap (F6). */
export const showFreeSpace = writable<boolean>(false);
/** Toggle display of unknown space in treemap (F7). */
export const showUnknown = writable<boolean>(false);
