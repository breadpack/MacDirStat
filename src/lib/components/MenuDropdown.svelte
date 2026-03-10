<script lang="ts">
  import { showFreeSpace, showUnknown } from "../stores/selectionStore";
  import { showExtensionPanel } from "../stores/extensionStore";
  import { layoutStore } from "../stores/layoutStore";
  import { tree, currentVolume } from "../stores/scanStore";
  import { zoomRoot } from "../stores/selectionStore";

  interface Props {
    optionsPanelOpen: boolean;
    onOpenCleanupSettings?: () => void;
    onOpenSettings?: () => void;
  }

  let { optionsPanelOpen = $bindable(false), onOpenCleanupSettings, onOpenSettings }: Props = $props();

  let menuOpen = $state(false);
  let menuEl: HTMLDivElement | undefined = $state();
  let hasTree = $derived(!!$tree);

  function toggleMenu() {
    menuOpen = !menuOpen;
  }

  function handleClickOutside(e: MouseEvent) {
    if (menuEl && !menuEl.contains(e.target as Node)) {
      menuOpen = false;
    }
  }

  $effect(() => {
    if (menuOpen) {
      document.addEventListener("click", handleClickOutside, true);
      return () => document.removeEventListener("click", handleClickOutside, true);
    }
  });
</script>

<div class="menu-wrapper" bind:this={menuEl}>
  <button class="menu-btn" onclick={toggleMenu} class:active={menuOpen}>
    Menu &#x25BE;
  </button>
  {#if menuOpen}
    <div class="menu-dropdown">
      <div class="menu-section-label">View</div>
      <button class="menu-item" onclick={() => { layoutStore.toggleTree(); }}>
        <span class="check">{$layoutStore.showTree ? "\u2713" : ""}</span>
        Tree Panel
      </button>
      <button
        class="menu-item"
        onclick={() => { layoutStore.toggleExtensions(); }}
        disabled={!hasTree}
      >
        <span class="check">{$showExtensionPanel ? "\u2713" : ""}</span>
        Extensions
        <span class="shortcut">F8</span>
      </button>
      <button class="menu-item" onclick={() => { layoutStore.toggleTreemap(); }}>
        <span class="check">{$layoutStore.showTreemap ? "\u2713" : ""}</span>
        Treemap
        <span class="shortcut">F9</span>
      </button>
      <div class="menu-separator"></div>
      <button
        class="menu-item"
        onclick={() => { showFreeSpace.update(v => !v); }}
        disabled={!hasTree || !$currentVolume || !!$zoomRoot}
      >
        <span class="check">{$showFreeSpace ? "\u2713" : ""}</span>
        Free Space
        <span class="shortcut">F6</span>
      </button>
      <button
        class="menu-item"
        onclick={() => { showUnknown.update(v => !v); }}
        disabled={!hasTree || !$currentVolume || !!$zoomRoot}
      >
        <span class="check">{$showUnknown ? "\u2713" : ""}</span>
        Unknown Space
        <span class="shortcut">F7</span>
      </button>
      <button class="menu-item" onclick={() => { optionsPanelOpen = !optionsPanelOpen; }}>
        <span class="check">{optionsPanelOpen ? "\u2713" : ""}</span>
        Treemap Options
      </button>
      <div class="menu-separator"></div>
      <button class="menu-item" onclick={() => { onOpenCleanupSettings?.(); menuOpen = false; }}>
        <span class="check"></span>
        Cleanup Actions...
      </button>
      <button class="menu-item" onclick={() => { onOpenSettings?.(); menuOpen = false; }}>
        <span class="check"></span>
        Settings...
        <span class="shortcut">{"\u2318"},</span>
      </button>
    </div>
  {/if}
</div>

<style>
  .menu-wrapper {
    position: relative;
  }

  .menu-btn {
    padding: 6px 16px;
    border: 1px solid #555;
    border-radius: 4px;
    background: #2d2d2d;
    color: #ccc;
    cursor: pointer;
    font-size: 13px;
  }

  .menu-btn:hover {
    background: #3d3d3d;
  }

  .menu-btn.active {
    background: #4A90D9;
    border-color: #4A90D9;
    color: #fff;
  }

  .menu-dropdown {
    position: absolute;
    top: 100%;
    right: 0;
    margin-top: 4px;
    min-width: 220px;
    background: #2d2d2d;
    border: 1px solid #555;
    border-radius: 6px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.4);
    padding: 4px 0;
    z-index: 1000;
  }

  .menu-section-label {
    padding: 4px 12px;
    font-size: 11px;
    color: #888;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .menu-item {
    display: flex;
    align-items: center;
    width: 100%;
    padding: 6px 12px;
    border: none;
    background: none;
    color: #ccc;
    font-size: 13px;
    cursor: pointer;
    text-align: left;
  }

  .menu-item:hover:not(:disabled) {
    background: #3d3d3d;
  }

  .menu-item:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .check {
    width: 20px;
    text-align: center;
    flex-shrink: 0;
  }

  .shortcut {
    margin-left: auto;
    color: #666;
    font-size: 12px;
  }

  .menu-separator {
    height: 1px;
    background: #444;
    margin: 4px 0;
  }
</style>
