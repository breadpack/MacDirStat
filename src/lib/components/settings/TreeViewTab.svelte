<script lang="ts">
  import { settings } from "../../stores/settingsStore";
  import { columns, sortState } from "../../stores/columnStore";

  const COLUMN_LABELS: Record<string, string> = {
    name: "Name",
    size: "Size",
    percent: "Percentage",
    files: "Files",
    subdirs: "Subdirs",
    modified: "Last Modified",
    attributes: "Attributes",
  };

  const SORT_OPTIONS = [
    { value: "name", label: "Name" },
    { value: "size", label: "Size" },
    { value: "files", label: "File Count" },
    { value: "subdirs", label: "Subdir Count" },
    { value: "modified", label: "Last Modified" },
  ];

  function toggleColumn(id: string) {
    if (id === "name") return; // Name column is always visible
    columns.toggleVisibility(id);
    // Also sync to settings
    settings.update((s) => ({
      ...s,
      treeView: {
        ...s.treeView,
        showColumns: {
          ...s.treeView.showColumns,
          [id]: !s.treeView.showColumns[id],
        },
      },
    }));
  }

  function handleSortChange(e: Event) {
    const value = (e.target as HTMLSelectElement).value;
    sortState.set({
      columnId: value,
      direction: $sortState.direction,
    });
    settings.update((s) => ({
      ...s,
      treeView: { ...s.treeView, sortBy: value },
    }));
  }

  function handleSortDirChange(e: Event) {
    const checked = (e.target as HTMLInputElement).checked;
    sortState.set({
      columnId: $sortState.columnId,
      direction: checked ? "desc" : "asc",
    });
    settings.update((s) => ({
      ...s,
      treeView: { ...s.treeView, sortDesc: checked },
    }));
  }
</script>

<div class="tab-content">
  <div class="setting-group">
    <label class="group-label">Visible Columns</label>
    <div class="column-grid">
      {#each Object.entries(COLUMN_LABELS) as [id, label]}
        <label class="checkbox-label" class:disabled={id === "name"}>
          <input
            type="checkbox"
            checked={$settings.treeView.showColumns[id] !== false}
            disabled={id === "name"}
            onchange={() => toggleColumn(id)}
          />
          {label}
        </label>
      {/each}
    </div>
  </div>

  <div class="setting-group">
    <label class="group-label">Default Sort</label>
    <div class="sort-row">
      <select
        value={$sortState.columnId}
        onchange={handleSortChange}
      >
        {#each SORT_OPTIONS as opt}
          <option value={opt.value}>{opt.label}</option>
        {/each}
      </select>
      <label class="checkbox-label">
        <input
          type="checkbox"
          checked={$sortState.direction === "desc"}
          onchange={handleSortDirChange}
        />
        Descending
      </label>
    </div>
  </div>

  <div class="setting-group">
    <label class="checkbox-label">
      <input
        type="checkbox"
        checked={$settings.treeView.showHiddenFiles}
        onchange={(e) =>
          settings.update((s) => ({
            ...s,
            treeView: { ...s.treeView, showHiddenFiles: e.currentTarget.checked },
          }))}
      />
      Show hidden files (dotfiles)
    </label>
    <span class="hint">Hidden files are always scanned but can be filtered from display</span>
  </div>
</div>

<style>
  .tab-content {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .setting-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .group-label {
    font-size: 13px;
    font-weight: 600;
    color: #ddd;
  }

  .column-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 6px;
  }

  .checkbox-label {
    font-size: 13px;
    color: #ccc;
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
  }

  .checkbox-label.disabled {
    opacity: 0.5;
    cursor: default;
  }

  .sort-row {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  select {
    padding: 4px 8px;
    background: #2a2a2a;
    border: 1px solid #555;
    border-radius: 4px;
    color: #ccc;
    font-size: 13px;
  }

  .hint {
    font-size: 11px;
    color: #888;
  }
</style>
