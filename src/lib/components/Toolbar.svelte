<script lang="ts">
  import { stopScan, scanning, tree, currentVolume } from "../stores/scanStore";
  import { selectedPath, zoomRoot, showFreeSpace, showUnknown } from "../stores/selectionStore";
  import { showExtensionPanel } from "../stores/extensionStore";
  import { layoutStore } from "../stores/layoutStore";
  import { getParentPath, getLastChild } from "../stores/navigationStore";
  import { get } from "svelte/store";
  import { actionOpenFolder, actionRefresh } from "../utils/actions";

  interface Props {
    optionsPanelOpen: boolean;
    onSelectParent?: () => void;
    onReselectChild?: () => void;
    onOpenCleanupSettings?: () => void;
    onOpenSettings?: () => void;
  }

  let { optionsPanelOpen = $bindable(false), onSelectParent, onReselectChild, onOpenCleanupSettings, onOpenSettings }: Props = $props();

  let hasTree = $derived(!!$tree);

  function toggleOptions() {
    optionsPanelOpen = !optionsPanelOpen;
  }

  let canGoParent = $derived.by(() => {
    const sp = $selectedPath;
    if (!sp) return false;
    const t = $tree;
    if (!t) return false;
    const parentPath = getParentPath(sp);
    return parentPath !== null && parentPath.length >= t.path.length;
  });

  let canGoChild = $derived.by(() => {
    const sp = $selectedPath;
    if (!sp) return false;
    return getLastChild(sp) !== null;
  });
</script>

<div class="toolbar">
  <button onclick={actionOpenFolder} disabled={$scanning}>Open Folder</button>
  <button onclick={actionRefresh} disabled={$scanning || !hasTree} title="Refresh (F5)">Refresh</button>
  <button onclick={stopScan} disabled={!$scanning}>Stop</button>
  {#if $scanning}
    <span class="scanning-indicator">Scanning...</span>
  {/if}
  <span class="separator"></span>
  <button
    onclick={() => onSelectParent?.()}
    disabled={!canGoParent}
    title="Select Parent (Alt+Up)"
  >Parent</button>
  <button
    onclick={() => onReselectChild?.()}
    disabled={!canGoChild}
    title="Re-select Child (Alt+Down)"
  >Child</button>
  <span class="separator"></span>
  <button
    class:active={$showFreeSpace}
    onclick={() => showFreeSpace.update(v => !v)}
    disabled={!hasTree || !$currentVolume || !!$zoomRoot}
    title="Show Free Space (F6)"
  >Free</button>
  <button
    class:active={$showUnknown}
    onclick={() => showUnknown.update(v => !v)}
    disabled={!hasTree || !$currentVolume || !!$zoomRoot}
    title="Show Unknown Space (F7)"
  >Unknown</button>
  <span class="separator"></span>
  <button
    class:active={$layoutStore.showTree}
    onclick={() => layoutStore.toggleTree()}
    title="Toggle Tree Panel"
  >Tree</button>
  <button
    class:active={$showExtensionPanel}
    onclick={() => layoutStore.toggleExtensions()}
    disabled={!hasTree}
    title="Extension List (F8)"
  >Extensions</button>
  <button
    class:active={$layoutStore.showTreemap}
    onclick={() => layoutStore.toggleTreemap()}
    title="Toggle Treemap (F9)"
  >Treemap</button>
  <div class="spacer"></div>
  <button
    onclick={() => onOpenCleanupSettings?.()}
    title="Configure Cleanup Actions"
  >Cleanups</button>
  <button
    onclick={() => onOpenSettings?.()}
    title="Settings (Cmd+,)"
  >Settings</button>
  <button
    class="options-btn"
    class:active={optionsPanelOpen}
    onclick={toggleOptions}
    title="Treemap Options"
  >Options</button>
</div>

<style>
  .toolbar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border-color);
  }

  button {
    padding: 6px 16px;
    border: 1px solid var(--border-color-strong);
    border-radius: 4px;
    background: var(--bg-tertiary);
    color: var(--text-primary);
    cursor: pointer;
    font-size: 13px;
  }

  button:hover:not(:disabled) {
    background: var(--hover-bg);
  }

  button:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .scanning-indicator {
    color: #4ad94a;
    font-size: 13px;
    animation: pulse 1s infinite;
  }

  .separator {
    width: 1px;
    height: 20px;
    background: var(--border-color-strong);
    margin: 0 4px;
  }

  .spacer {
    flex: 1;
  }

  button.active {
    background: var(--accent-color);
    border-color: var(--accent-color);
    color: #fff;
  }

  .options-btn.active {
    background: var(--accent-color);
    border-color: var(--accent-color);
    color: #fff;
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.5; }
  }
</style>
