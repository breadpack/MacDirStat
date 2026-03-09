import { writable } from "svelte/store";

export interface ColumnDef {
  id: string;
  label: string;
  width: number;
  minWidth: number;
  visible: boolean;
  sortable: boolean;
  align: "left" | "right" | "center";
}

export type SortDirection = "asc" | "desc";

export interface SortState {
  columnId: string;
  direction: SortDirection;
}

const STORAGE_KEY = "macdirstat-columns";
const SORT_STORAGE_KEY = "macdirstat-sort";

function defaultColumns(): ColumnDef[] {
  return [
    { id: "name", label: "Name", width: 300, minWidth: 120, visible: true, sortable: true, align: "left" },
    { id: "size", label: "Size", width: 80, minWidth: 50, visible: true, sortable: true, align: "right" },
    { id: "percent", label: "%", width: 80, minWidth: 40, visible: true, sortable: false, align: "left" },
    { id: "files", label: "Files", width: 70, minWidth: 40, visible: true, sortable: true, align: "right" },
    { id: "subdirs", label: "Subdirs", width: 70, minWidth: 40, visible: true, sortable: true, align: "right" },
    { id: "modified", label: "Last Modified", width: 140, minWidth: 80, visible: true, sortable: true, align: "left" },
    { id: "attributes", label: "Attr", width: 60, minWidth: 40, visible: true, sortable: false, align: "center" },
  ];
}

function loadColumns(): ColumnDef[] {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored) {
      const parsed = JSON.parse(stored) as ColumnDef[];
      // Merge with defaults to handle new columns added in future updates
      const defaults = defaultColumns();
      return defaults.map((def) => {
        const saved = parsed.find((c) => c.id === def.id);
        if (saved) {
          return { ...def, width: saved.width, visible: saved.visible };
        }
        return def;
      });
    }
  } catch {
    // ignore
  }
  return defaultColumns();
}

function loadSort(): SortState {
  try {
    const stored = localStorage.getItem(SORT_STORAGE_KEY);
    if (stored) {
      return JSON.parse(stored) as SortState;
    }
  } catch {
    // ignore
  }
  return { columnId: "size", direction: "desc" };
}

function createColumnStore() {
  const { subscribe, set, update } = writable<ColumnDef[]>(loadColumns());

  subscribe((cols) => {
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(cols));
    } catch {
      // ignore
    }
  });

  return {
    subscribe,
    set,
    update,
    toggleVisibility(id: string) {
      update((cols) =>
        cols.map((c) => (c.id === id && c.id !== "name" ? { ...c, visible: !c.visible } : c))
      );
    },
    setWidth(id: string, width: number) {
      update((cols) =>
        cols.map((c) => (c.id === id ? { ...c, width: Math.max(c.minWidth, width) } : c))
      );
    },
    reset() {
      set(defaultColumns());
    },
  };
}

function createSortStore() {
  const { subscribe, set, update } = writable<SortState>(loadSort());

  subscribe((sort) => {
    try {
      localStorage.setItem(SORT_STORAGE_KEY, JSON.stringify(sort));
    } catch {
      // ignore
    }
  });

  return {
    subscribe,
    set,
    update,
    toggleSort(columnId: string) {
      update((s) => {
        if (s.columnId === columnId) {
          return { columnId, direction: s.direction === "asc" ? "desc" : "asc" };
        }
        // Default sort directions per column
        const defaultDesc = ["size", "files", "subdirs", "modified"];
        return { columnId, direction: defaultDesc.includes(columnId) ? "desc" : "asc" };
      });
    },
  };
}

export const columns = createColumnStore();
export const sortState = createSortStore();
