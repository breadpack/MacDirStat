import { writable } from "svelte/store";

export interface TreemapOptions {
  // Cushion shading
  cushionEnabled: boolean;
  brightness: number;      // 0.0 ~ 1.0
  cushionHeight: number;   // 0.0 ~ 1.0
  scaleFactor: number;     // 0.0 ~ 1.0
  ambientLight: number;    // 0.0 ~ 1.0

  // Light position (normalized direction)
  lightX: number;          // -1 ~ 1
  lightY: number;          // -1 ~ 1

  // Grid
  gridEnabled: boolean;
  gridColor: string;
  gridWidth: number;       // 0 ~ 5

  // Style preset
  style: "kdirstat" | "sequoiaview";

  // Layout
  padding: number;         // 0 ~ 5
}

export const DEFAULT_OPTIONS: TreemapOptions = {
  cushionEnabled: true,
  brightness: 0.84,
  cushionHeight: 0.40,
  scaleFactor: 0.90,
  ambientLight: 0.15,
  lightX: -1,
  lightY: -1,
  gridEnabled: true,
  gridColor: "#000000",
  gridWidth: 1,
  style: "sequoiaview",
  padding: 1,
};

const KDIRSTAT_PRESET: Partial<TreemapOptions> = {
  cushionEnabled: true,
  brightness: 0.90,
  cushionHeight: 0.25,
  scaleFactor: 0.85,
  ambientLight: 0.20,
  gridEnabled: true,
  gridWidth: 2,
  gridColor: "#000000",
  padding: 2,
  style: "kdirstat",
};

const SEQUOIAVIEW_PRESET: Partial<TreemapOptions> = {
  cushionEnabled: true,
  brightness: 0.84,
  cushionHeight: 0.50,
  scaleFactor: 0.90,
  ambientLight: 0.10,
  gridEnabled: true,
  gridWidth: 1,
  gridColor: "#000000",
  padding: 1,
  style: "sequoiaview",
};

const STORAGE_KEY = "macdirstat-treemap-options";

function loadFromStorage(): TreemapOptions {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored) {
      const parsed = JSON.parse(stored);
      return { ...DEFAULT_OPTIONS, ...parsed };
    }
  } catch {
    // ignore
  }
  return { ...DEFAULT_OPTIONS };
}

function saveToStorage(options: TreemapOptions): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(options));
  } catch {
    // ignore
  }
}

function createTreemapOptionsStore() {
  const { subscribe, set, update } = writable<TreemapOptions>(loadFromStorage());

  // Auto-save on changes
  let initialized = false;
  subscribe((value) => {
    if (initialized) {
      saveToStorage(value);
    }
    initialized = true;
  });

  return {
    subscribe,
    set,
    update,
    applyPreset(preset: "kdirstat" | "sequoiaview") {
      update((opts) => {
        const presetValues = preset === "kdirstat" ? KDIRSTAT_PRESET : SEQUOIAVIEW_PRESET;
        return { ...opts, ...presetValues };
      });
    },
    reset() {
      set({ ...DEFAULT_OPTIONS });
    },
  };
}

export const treemapOptions = createTreemapOptionsStore();
