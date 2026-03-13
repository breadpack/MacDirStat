/**
 * Settings store using tauri-plugin-store for persistence.
 * Falls back to localStorage if the plugin is not available.
 */
import { writable, get } from "svelte/store";
import type { AppSettings } from "../settings";
import { DEFAULT_SETTINGS } from "../settings";

const STORE_FILE = "settings.json";
const STORE_KEY = "app-settings";
const LS_FALLBACK_KEY = "macdirstat-settings";

function deepMerge<T extends object>(defaults: T, partial: Partial<T>): T {
  const result = { ...defaults };
  for (const key of Object.keys(defaults) as (keyof T)[]) {
    if (
      partial[key] !== undefined &&
      typeof defaults[key] === "object" &&
      defaults[key] !== null &&
      !Array.isArray(defaults[key])
    ) {
      result[key] = deepMerge(
        defaults[key] as object,
        partial[key] as object,
      ) as T[keyof T];
    } else if (partial[key] !== undefined) {
      result[key] = partial[key] as T[keyof T];
    }
  }
  return result;
}

import type { Store } from "@tauri-apps/plugin-store";

let storeInstance: Store | null = null;

async function getStore() {
  if (storeInstance) return storeInstance;
  try {
    const { load } = await import("@tauri-apps/plugin-store");
    storeInstance = await load(STORE_FILE);
    return storeInstance;
  } catch {
    return null;
  }
}

async function loadSettings(): Promise<AppSettings> {
  try {
    const store = await getStore();
    if (store) {
      const saved = await store.get<AppSettings>(STORE_KEY);
      if (saved) {
        return deepMerge(DEFAULT_SETTINGS, saved as Partial<AppSettings>);
      }
    }
  } catch {
    // fallback to localStorage
  }
  try {
    const raw = localStorage.getItem(LS_FALLBACK_KEY);
    if (raw) {
      return deepMerge(DEFAULT_SETTINGS, JSON.parse(raw));
    }
  } catch {
    // ignore
  }
  return { ...DEFAULT_SETTINGS };
}

async function persistSettings(settings: AppSettings): Promise<void> {
  try {
    const store = await getStore();
    if (store) {
      await store.set(STORE_KEY, settings);
      await store.save();
    }
  } catch {
    // ignore
  }
  try {
    localStorage.setItem(LS_FALLBACK_KEY, JSON.stringify(settings));
  } catch {
    // ignore
  }
}

function createSettingsStore() {
  const { subscribe, set, update } = writable<AppSettings>(
    deepMerge(DEFAULT_SETTINGS, {}),
  );

  let initialized = false;
  let saveTimer: ReturnType<typeof setTimeout> | null = null;

  // Debounced save
  subscribe((value) => {
    if (!initialized) return;
    if (saveTimer) clearTimeout(saveTimer);
    saveTimer = setTimeout(() => persistSettings(value), 300);
  });

  return {
    subscribe,
    set,
    update,
    async init() {
      const loaded = await loadSettings();
      set(loaded);
      initialized = true;
    },
    reset() {
      set(deepMerge(DEFAULT_SETTINGS, {}));
    },
    resetSection(section: keyof AppSettings) {
      update((s) => ({ ...s, [section]: { ...DEFAULT_SETTINGS[section] } }));
    },
  };
}

export const settings = createSettingsStore();
