<script lang="ts">
  import { stopScan, scanning, tree } from "../stores/scanStore";
  import { selectedPath } from "../stores/selectionStore";
  import { getParentPath, getLastChild } from "../stores/navigationStore";
  import { actionOpenFolder, actionRefresh } from "../utils/actions";
  import MenuDropdown from "./MenuDropdown.svelte";

  interface Props {
    optionsPanelOpen: boolean;
    onSelectParent?: () => void;
    onReselectChild?: () => void;
    onOpenCleanupSettings?: () => void;
    onOpenSettings?: () => void;
    onOpenCleanupRecommendations?: () => void;
  }

  let {
    optionsPanelOpen = $bindable(false),
    onSelectParent,
    onReselectChild,
    onOpenCleanupSettings,
    onOpenSettings,
    onOpenCleanupRecommendations,
  }: Props = $props();

  let hasTree = $derived(!!$tree);

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
  <div class="spacer"></div>
  <MenuDropdown
    bind:optionsPanelOpen
    {onOpenCleanupSettings}
    {onOpenSettings}
    {onOpenCleanupRecommendations}
  />
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
