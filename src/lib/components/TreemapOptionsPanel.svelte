<script lang="ts">
  import { treemapOptions, DEFAULT_OPTIONS, type TreemapOptions } from "../stores/treemapOptionsStore";

  let { onclose }: { onclose: () => void } = $props();

  let opts = $state<TreemapOptions>({ ...DEFAULT_OPTIONS });
  let debounceTimer: ReturnType<typeof setTimeout> | null = null;

  // Sync from store
  const unsub = treemapOptions.subscribe((v) => {
    opts = { ...v };
  });

  function commitDebounced() {
    if (debounceTimer) clearTimeout(debounceTimer);
    debounceTimer = setTimeout(() => {
      treemapOptions.set({ ...opts });
    }, 50);
  }

  function commitImmediate() {
    if (debounceTimer) clearTimeout(debounceTimer);
    treemapOptions.set({ ...opts });
  }

  function applyPreset(preset: "kdirstat" | "sequoiaview") {
    treemapOptions.applyPreset(preset);
  }

  function resetDefaults() {
    treemapOptions.reset();
  }

  // Light position drag
  let lightDragging = $state(false);

  function lightPadXY(e: MouseEvent, el: HTMLElement) {
    const rect = el.getBoundingClientRect();
    const x = ((e.clientX - rect.left) / rect.width) * 2 - 1;
    const y = ((e.clientY - rect.top) / rect.height) * 2 - 1;
    opts.lightX = Math.max(-1, Math.min(1, x));
    opts.lightY = Math.max(-1, Math.min(1, y));
    commitDebounced();
  }

  function handleLightPointerDown(e: MouseEvent) {
    lightDragging = true;
    const el = e.currentTarget as HTMLElement;
    lightPadXY(e, el);

    function onMove(ev: MouseEvent) {
      lightPadXY(ev, el);
    }
    function onUp() {
      lightDragging = false;
      commitImmediate();
      window.removeEventListener("mousemove", onMove);
      window.removeEventListener("mouseup", onUp);
    }
    window.addEventListener("mousemove", onMove);
    window.addEventListener("mouseup", onUp);
  }

  // Derived light dot position (0..1 range for CSS)
  let lightDotLeft = $derived(((opts.lightX + 1) / 2) * 100);
  let lightDotTop = $derived(((opts.lightY + 1) / 2) * 100);

  // Cleanup
  $effect(() => {
    return () => {
      unsub();
      if (debounceTimer) clearTimeout(debounceTimer);
    };
  });
</script>

<div class="options-panel">
  <div class="panel-header">
    <span class="panel-title">Treemap Options</span>
    <button class="close-btn" onclick={onclose}>&times;</button>
  </div>

  <!-- Style Presets -->
  <div class="section">
    <div class="section-label">Style</div>
    <div class="preset-buttons">
      <button
        class="preset-btn"
        class:active={opts.style === "kdirstat"}
        onclick={() => applyPreset("kdirstat")}
      >KDirStat</button>
      <button
        class="preset-btn"
        class:active={opts.style === "sequoiaview"}
        onclick={() => applyPreset("sequoiaview")}
      >SequoiaView</button>
    </div>
  </div>

  <!-- Cushion Shading -->
  <div class="section">
    <label class="checkbox-row">
      <input
        type="checkbox"
        bind:checked={opts.cushionEnabled}
        onchange={commitImmediate}
      />
      Cushion Shading
    </label>

    {#if opts.cushionEnabled}
      <div class="slider-group">
        <div class="slider-row">
          <span class="slider-label">Brightness</span>
          <input
            type="range" min="0" max="1" step="0.01"
            bind:value={opts.brightness}
            oninput={commitDebounced}
          />
          <span class="slider-value">{opts.brightness.toFixed(2)}</span>
        </div>
        <div class="slider-row">
          <span class="slider-label">Height</span>
          <input
            type="range" min="0" max="1" step="0.01"
            bind:value={opts.cushionHeight}
            oninput={commitDebounced}
          />
          <span class="slider-value">{opts.cushionHeight.toFixed(2)}</span>
        </div>
        <div class="slider-row">
          <span class="slider-label">Scale Factor</span>
          <input
            type="range" min="0" max="1" step="0.01"
            bind:value={opts.scaleFactor}
            oninput={commitDebounced}
          />
          <span class="slider-value">{opts.scaleFactor.toFixed(2)}</span>
        </div>
        <div class="slider-row">
          <span class="slider-label">Ambient Light</span>
          <input
            type="range" min="0" max="1" step="0.01"
            bind:value={opts.ambientLight}
            oninput={commitDebounced}
          />
          <span class="slider-value">{opts.ambientLight.toFixed(2)}</span>
        </div>
      </div>

      <!-- Light Position -->
      <div class="light-section">
        <span class="slider-label">Light Position</span>
        <!-- svelte-ignore a11y_role_has_required_aria_props -->
        <div
          class="light-pad"
          role="slider"
          tabindex="0"
          aria-label="Light position"
          aria-valuetext="X: {opts.lightX.toFixed(2)}, Y: {opts.lightY.toFixed(2)}"
          onmousedown={handleLightPointerDown}
        >
          <div class="light-crosshair-h"></div>
          <div class="light-crosshair-v"></div>
          <div
            class="light-dot"
            style="left: {lightDotLeft}%; top: {lightDotTop}%"
          ></div>
        </div>
      </div>
    {/if}
  </div>

  <!-- Grid -->
  <div class="section">
    <label class="checkbox-row">
      <input
        type="checkbox"
        bind:checked={opts.gridEnabled}
        onchange={commitImmediate}
      />
      Grid
    </label>

    {#if opts.gridEnabled}
      <div class="slider-group">
        <div class="slider-row">
          <span class="slider-label">Color</span>
          <input
            type="color"
            bind:value={opts.gridColor}
            onchange={commitImmediate}
            class="color-input"
          />
        </div>
        <div class="slider-row">
          <span class="slider-label">Width</span>
          <input
            type="range" min="0" max="5" step="1"
            bind:value={opts.gridWidth}
            oninput={commitDebounced}
          />
          <span class="slider-value">{opts.gridWidth}</span>
        </div>
      </div>
    {/if}
  </div>

  <!-- Layout -->
  <div class="section">
    <div class="slider-group">
      <div class="slider-row">
        <span class="slider-label">Padding</span>
        <input
          type="range" min="0" max="5" step="1"
          bind:value={opts.padding}
          oninput={commitDebounced}
        />
        <span class="slider-value">{opts.padding}</span>
      </div>
    </div>
  </div>

  <!-- Reset -->
  <div class="section">
    <button class="reset-btn" onclick={resetDefaults}>Reset to Defaults</button>
  </div>
</div>

<style>
  .options-panel {
    position: absolute;
    top: 8px;
    right: 8px;
    width: 260px;
    background: rgba(30, 30, 30, 0.96);
    border: 1px solid #444;
    border-radius: 6px;
    z-index: 50;
    font-size: 12px;
    color: #ccc;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.5);
    max-height: calc(100% - 16px);
    overflow-y: auto;
  }

  .panel-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 10px;
    border-bottom: 1px solid #444;
  }

  .panel-title {
    font-weight: 600;
    font-size: 13px;
    color: #eee;
  }

  .close-btn {
    background: none;
    border: none;
    color: #888;
    font-size: 18px;
    cursor: pointer;
    padding: 0 4px;
    line-height: 1;
  }

  .close-btn:hover {
    color: #fff;
  }

  .section {
    padding: 8px 10px;
    border-bottom: 1px solid #333;
  }

  .section:last-child {
    border-bottom: none;
  }

  .section-label {
    font-weight: 600;
    margin-bottom: 6px;
    color: #aaa;
  }

  .preset-buttons {
    display: flex;
    gap: 4px;
  }

  .preset-btn {
    flex: 1;
    padding: 5px 8px;
    border: 1px solid #555;
    border-radius: 3px;
    background: #2d2d2d;
    color: #ccc;
    cursor: pointer;
    font-size: 12px;
  }

  .preset-btn:hover {
    background: #3d3d3d;
  }

  .preset-btn.active {
    background: #4A90D9;
    border-color: #4A90D9;
    color: #fff;
  }

  .checkbox-row {
    display: flex;
    align-items: center;
    gap: 6px;
    cursor: pointer;
    font-weight: 600;
    color: #aaa;
  }

  .checkbox-row input[type="checkbox"] {
    margin: 0;
  }

  .slider-group {
    margin-top: 6px;
  }

  .slider-row {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-bottom: 4px;
  }

  .slider-label {
    width: 80px;
    flex-shrink: 0;
    color: #999;
  }

  .slider-row input[type="range"] {
    flex: 1;
    height: 4px;
    accent-color: #4A90D9;
  }

  .slider-value {
    width: 32px;
    text-align: right;
    font-family: monospace;
    font-size: 11px;
    color: #888;
  }

  .color-input {
    width: 28px;
    height: 22px;
    border: 1px solid #555;
    border-radius: 3px;
    padding: 0;
    cursor: pointer;
    background: none;
  }

  .light-section {
    margin-top: 8px;
  }

  .light-pad {
    width: 80px;
    height: 80px;
    background: #222;
    border: 1px solid #555;
    border-radius: 3px;
    margin-top: 4px;
    position: relative;
    cursor: crosshair;
    overflow: hidden;
  }

  .light-crosshair-h,
  .light-crosshair-v {
    position: absolute;
    background: #444;
  }

  .light-crosshair-h {
    top: 50%;
    left: 0;
    right: 0;
    height: 1px;
  }

  .light-crosshair-v {
    left: 50%;
    top: 0;
    bottom: 0;
    width: 1px;
  }

  .light-dot {
    position: absolute;
    width: 10px;
    height: 10px;
    background: #4A90D9;
    border: 1px solid #fff;
    border-radius: 50%;
    transform: translate(-50%, -50%);
    pointer-events: none;
  }

  .reset-btn {
    width: 100%;
    padding: 6px;
    border: 1px solid #555;
    border-radius: 3px;
    background: #2d2d2d;
    color: #ccc;
    cursor: pointer;
    font-size: 12px;
  }

  .reset-btn:hover {
    background: #3d3d3d;
  }
</style>
