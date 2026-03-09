<script lang="ts">
  import { settings } from "../../stores/settingsStore";

  function handleThemeChange(e: Event) {
    const value = (e.target as HTMLSelectElement).value as "dark" | "light" | "system";
    settings.update((s) => ({
      ...s,
      appearance: { ...s.appearance, theme: value },
    }));
  }

  function handlePanelWidthChange(e: Event) {
    const val = parseInt((e.target as HTMLInputElement).value) || 35;
    settings.update((s) => ({
      ...s,
      appearance: { ...s.appearance, treePanelWidth: val },
    }));
  }
</script>

<div class="tab-content">
  <div class="setting-group">
    <label class="group-label">Theme</label>
    <select value={$settings.appearance.theme} onchange={handleThemeChange}>
      <option value="dark">Dark</option>
      <option value="light">Light</option>
      <option value="system">System</option>
    </select>
    <span class="hint">
      {#if $settings.appearance.theme === "system"}
        Follows your macOS appearance setting
      {:else if $settings.appearance.theme === "light"}
        Light mode
      {:else}
        Dark mode (default)
      {/if}
    </span>
  </div>

  <div class="setting-group">
    <label class="group-label">Tree Panel Width</label>
    <div class="slider-row">
      <input
        type="range"
        min="20"
        max="60"
        step="1"
        value={$settings.appearance.treePanelWidth}
        oninput={handlePanelWidthChange}
      />
      <span class="slider-value">{$settings.appearance.treePanelWidth}%</span>
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

  .group-label {
    font-size: 13px;
    font-weight: 600;
    color: #ddd;
  }

  select {
    width: 160px;
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

  .slider-row {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .slider-row input[type="range"] {
    flex: 1;
    max-width: 200px;
    accent-color: #4A90D9;
  }

  .slider-value {
    font-size: 13px;
    color: #ccc;
    min-width: 40px;
  }
</style>
