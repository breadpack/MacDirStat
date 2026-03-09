import { writable, get } from "svelte/store";

export interface PanelLayout {
  showTree: boolean;
  showExtensions: boolean;
  showTreemap: boolean;
  treePanelWidth: number; // px
  extensionPanelWidth: number; // px
}

const STORAGE_KEY = "macdirstat-panel-layout";

const DEFAULT_LAYOUT: PanelLayout = {
  showTree: true,
  showExtensions: false,
  showTreemap: true,
  treePanelWidth: 350,
  extensionPanelWidth: 250,
};

function loadLayout(): PanelLayout {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored) {
      const parsed = JSON.parse(stored);
      return { ...DEFAULT_LAYOUT, ...parsed };
    }
  } catch {
    // ignore
  }
  return { ...DEFAULT_LAYOUT };
}

function createLayoutStore() {
  const { subscribe, set, update } = writable<PanelLayout>(loadLayout());

  let saveTimer: ReturnType<typeof setTimeout> | null = null;

  function persistDebounced(layout: PanelLayout) {
    if (saveTimer) clearTimeout(saveTimer);
    saveTimer = setTimeout(() => {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(layout));
    }, 500);
  }

  // Auto-persist on changes
  subscribe((layout) => {
    persistDebounced(layout);
  });

  function ensureOneVisible(layout: PanelLayout): PanelLayout {
    if (!layout.showTree && !layout.showExtensions && !layout.showTreemap) {
      layout.showTree = true;
    }
    return layout;
  }

  return {
    subscribe,
    set,
    update,

    toggleTree() {
      update((l) => ensureOneVisible({ ...l, showTree: !l.showTree }));
    },

    toggleExtensions() {
      update((l) => ensureOneVisible({ ...l, showExtensions: !l.showExtensions }));
    },

    toggleTreemap() {
      update((l) => ensureOneVisible({ ...l, showTreemap: !l.showTreemap }));
    },

    setTreeWidth(width: number) {
      update((l) => {
        const maxWidth = Math.floor(window.innerWidth * 0.6);
        const clamped = Math.max(200, Math.min(width, maxWidth));
        return { ...l, treePanelWidth: clamped };
      });
    },

    setExtensionWidth(width: number) {
      update((l) => {
        const clamped = Math.max(150, Math.min(width, 400));
        return { ...l, extensionPanelWidth: clamped };
      });
    },
  };
}

export const layoutStore = createLayoutStore();
