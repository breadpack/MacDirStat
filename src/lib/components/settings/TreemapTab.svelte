<script lang="ts">
  import { settings } from "../../stores/settingsStore";
  import { treemapOptions } from "../../stores/treemapOptionsStore";

  // Sync treemap padding from settings to treemapOptions store
  function handlePaddingChange(e: Event) {
    const val = parseInt((e.target as HTMLInputElement).value) || 1;
    settings.update((s) => ({
      ...s,
      treemap: { ...s.treemap, padding: val },
    }));
    treemapOptions.update((opts) => ({ ...opts, padding: val }));
  }

  function handleShowLabelsChange(e: Event) {
    const checked = (e.target as HTMLInputElement).checked;
    settings.update((s) => ({
      ...s,
      treemap: { ...s.treemap, showLabels: checked },
    }));
  }

  function handleShowCleanupOverlayChange(e: Event) {
    const checked = (e.target as HTMLInputElement).checked;
    treemapOptions.update((opts) => ({ ...opts, showCleanupOverlay: checked }));
  }
</script>

<div class="tab-content">
  <div class="setting-group">
    <label class="group-label">Layout</label>
    <div class="field-row">
      <span class="field-label">Algorithm</span>
      <span class="field-value">Squarify (d3-hierarchy)</span>
    </div>
  </div>

  <div class="setting-group">
    <label class="group-label">Cell Padding</label>
    <div class="slider-row">
      <input
        type="range"
        min="0"
        max="5"
        step="1"
        value={$settings.treemap.padding}
        oninput={handlePaddingChange}
      />
      <span class="slider-value">{$settings.treemap.padding}px</span>
    </div>
  </div>

  <div class="setting-group">
    <label class="checkbox-label">
      <input
        type="checkbox"
        checked={$settings.treemap.showLabels}
        onchange={handleShowLabelsChange}
      />
      Show file labels in cells
    </label>
  </div>

  <div class="setting-group">
    <label class="checkbox-label">
      <input
        type="checkbox"
        checked={$treemapOptions.showCleanupOverlay}
        onchange={handleShowCleanupOverlayChange}
      />
      Show cleanup overlay (hatching on cleanable items)
    </label>
  </div>

  <div class="setting-group">
    <label class="group-label">Visual Style</label>
    <p class="hint">
      For detailed cushion shading, brightness, and grid options,
      use the Treemap Options panel (Options button in toolbar).
    </p>
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

  .field-row {
    display: flex;
    align-items: center;
    gap: 12px;
    font-size: 13px;
  }

  .field-label {
    color: #999;
  }

  .field-value {
    color: #ccc;
  }

  .slider-row {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .slider-row input[type="range"] {
    flex: 1;
    accent-color: #4A90D9;
  }

  .slider-value {
    font-size: 13px;
    color: #ccc;
    min-width: 30px;
  }

  .checkbox-label {
    font-size: 13px;
    color: #ccc;
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
  }

  .hint {
    font-size: 12px;
    color: #888;
    margin: 0;
  }
</style>
