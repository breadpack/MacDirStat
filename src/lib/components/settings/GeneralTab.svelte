<script lang="ts">
  import { settings } from "../../stores/settingsStore";
  import { DEFAULT_EXCLUDE_PATHS, DEFAULT_EXCLUDE_NAMES } from "../../settings";

  let newExcludePath = $state("");
  let newExcludeName = $state("");

  function addExcludePath() {
    const val = newExcludePath.trim();
    if (!val) return;
    settings.update((s) => ({
      ...s,
      general: {
        ...s.general,
        excludePaths: [...s.general.excludePaths, val],
      },
    }));
    newExcludePath = "";
  }

  function removeExcludePath(index: number) {
    settings.update((s) => ({
      ...s,
      general: {
        ...s.general,
        excludePaths: s.general.excludePaths.filter((_, i) => i !== index),
      },
    }));
  }

  function addExcludeName() {
    const val = newExcludeName.trim();
    if (!val) return;
    settings.update((s) => ({
      ...s,
      general: {
        ...s.general,
        excludeNames: [...s.general.excludeNames, val],
      },
    }));
    newExcludeName = "";
  }

  function removeExcludeName(index: number) {
    settings.update((s) => ({
      ...s,
      general: {
        ...s.general,
        excludeNames: s.general.excludeNames.filter((_, i) => i !== index),
      },
    }));
  }

  function resetExcludes() {
    settings.update((s) => ({
      ...s,
      general: {
        ...s.general,
        excludePaths: [...DEFAULT_EXCLUDE_PATHS],
        excludeNames: [...DEFAULT_EXCLUDE_NAMES],
      },
    }));
  }
</script>

<div class="tab-content">
  <div class="setting-group">
    <label class="checkbox-label">
      <input
        type="checkbox"
        checked={$settings.general.followSymlinks}
        onchange={(e) =>
          settings.update((s) => ({
            ...s,
            general: { ...s.general, followSymlinks: e.currentTarget.checked },
          }))}
      />
      Follow symbolic links
    </label>
  </div>

  <div class="setting-group">
    <label class="group-label">Max Children per Directory</label>
    <input
      type="number"
      min="50"
      max="10000"
      value={$settings.general.maxChildrenPerDir}
      onchange={(e) =>
        settings.update((s) => ({
          ...s,
          general: { ...s.general, maxChildrenPerDir: parseInt(e.currentTarget.value) || 200 },
        }))}
    />
    <span class="hint">Files beyond this limit are grouped as "other files"</span>
  </div>

  <div class="setting-group">
    <div class="group-header">
      <label class="group-label">Exclude Paths</label>
      <button class="reset-btn" onclick={resetExcludes}>Reset to Defaults</button>
    </div>
    <div class="list-container">
      {#each $settings.general.excludePaths as path, i}
        <div class="list-item">
          <span class="item-text">{path}</span>
          <button class="remove-btn" onclick={() => removeExcludePath(i)}>x</button>
        </div>
      {/each}
    </div>
    <div class="add-row">
      <input
        type="text"
        placeholder="/path/to/exclude"
        bind:value={newExcludePath}
        onkeydown={(e) => e.key === "Enter" && addExcludePath()}
      />
      <button onclick={addExcludePath}>Add</button>
    </div>
  </div>

  <div class="setting-group">
    <label class="group-label">Exclude Directory Names</label>
    <div class="list-container">
      {#each $settings.general.excludeNames as name, i}
        <div class="list-item">
          <span class="item-text">{name}</span>
          <button class="remove-btn" onclick={() => removeExcludeName(i)}>x</button>
        </div>
      {/each}
    </div>
    <div class="add-row">
      <input
        type="text"
        placeholder="directory-name"
        bind:value={newExcludeName}
        onkeydown={(e) => e.key === "Enter" && addExcludeName()}
      />
      <button onclick={addExcludeName}>Add</button>
    </div>
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

  .group-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .group-label {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-heading);
  }

  .checkbox-label {
    font-size: 13px;
    color: var(--text-primary);
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
  }

  .hint {
    font-size: 11px;
    color: var(--text-secondary);
  }

  input[type="number"] {
    width: 100px;
    padding: 4px 8px;
    background: var(--bg-input);
    border: 1px solid var(--border-color-strong);
    border-radius: 4px;
    color: var(--text-primary);
    font-size: 13px;
  }

  .list-container {
    max-height: 120px;
    overflow-y: auto;
    background: var(--bg-header);
    border: 1px solid var(--border-color-strong);
    border-radius: 4px;
    padding: 4px;
  }

  .list-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 2px 6px;
    font-size: 12px;
    color: var(--text-primary);
  }

  .list-item:hover {
    background: var(--bg-tertiary);
  }

  .item-text {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
  }

  .remove-btn {
    background: none;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    padding: 0 4px;
    font-size: 12px;
  }

  .remove-btn:hover {
    color: var(--danger-color);
  }

  .add-row {
    display: flex;
    gap: 6px;
  }

  .add-row input {
    flex: 1;
    padding: 4px 8px;
    background: var(--bg-input);
    border: 1px solid var(--border-color-strong);
    border-radius: 4px;
    color: var(--text-primary);
    font-size: 12px;
  }

  .add-row button {
    padding: 4px 12px;
    background: var(--bg-tertiary);
    border: 1px solid var(--border-color-strong);
    border-radius: 4px;
    color: var(--text-primary);
    cursor: pointer;
    font-size: 12px;
  }

  .add-row button:hover {
    background: var(--hover-bg);
  }

  .reset-btn {
    padding: 2px 8px;
    background: none;
    border: 1px solid var(--border-color-strong);
    border-radius: 4px;
    color: var(--text-secondary);
    cursor: pointer;
    font-size: 11px;
  }

  .reset-btn:hover {
    background: var(--bg-tertiary);
    color: var(--text-primary);
  }
</style>
