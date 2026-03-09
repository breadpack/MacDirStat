<script lang="ts">
  import { settings } from "../stores/settingsStore";
  import GeneralTab from "./settings/GeneralTab.svelte";
  import TreeViewTab from "./settings/TreeViewTab.svelte";
  import TreemapTab from "./settings/TreemapTab.svelte";
  import AppearanceTab from "./settings/AppearanceTab.svelte";

  interface Props {
    onClose: () => void;
  }

  let { onClose }: Props = $props();

  type TabId = "general" | "treeview" | "treemap" | "appearance";

  const TABS: { id: TabId; label: string }[] = [
    { id: "general", label: "General" },
    { id: "treeview", label: "TreeView" },
    { id: "treemap", label: "Treemap" },
    { id: "appearance", label: "Appearance" },
  ];

  let activeTab: TabId = $state("general");

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      onClose();
    }
  }

  function handleBackdropClick(e: MouseEvent) {
    if ((e.target as HTMLElement).classList.contains("dialog-backdrop")) {
      onClose();
    }
  }

  function resetAll() {
    settings.reset();
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  class="dialog-backdrop"
  role="dialog"
  aria-modal="true"
  aria-label="Settings"
  onclick={handleBackdropClick}
  onkeydown={handleKeydown}
>
  <div class="dialog">
    <div class="dialog-header">
      <h2>Settings</h2>
      <button class="close-btn" onclick={onClose}>x</button>
    </div>

    <div class="tabs">
      {#each TABS as tab}
        <button
          class="tab-btn"
          class:active={activeTab === tab.id}
          onclick={() => (activeTab = tab.id)}
        >
          {tab.label}
        </button>
      {/each}
    </div>

    <div class="tab-body">
      {#if activeTab === "general"}
        <GeneralTab />
      {:else if activeTab === "treeview"}
        <TreeViewTab />
      {:else if activeTab === "treemap"}
        <TreemapTab />
      {:else if activeTab === "appearance"}
        <AppearanceTab />
      {/if}
    </div>

    <div class="dialog-footer">
      <button class="reset-btn" onclick={resetAll}>Reset to Defaults</button>
      <button class="close-footer-btn" onclick={onClose}>Close</button>
    </div>
  </div>
</div>

<style>
  .dialog-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 200;
  }

  .dialog {
    background: #1e1e1e;
    border: 1px solid #444;
    border-radius: 8px;
    width: 520px;
    max-height: 80vh;
    display: flex;
    flex-direction: column;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
  }

  .dialog-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 16px;
    border-bottom: 1px solid #333;
  }

  .dialog-header h2 {
    margin: 0;
    font-size: 16px;
    font-weight: 600;
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
    color: #ccc;
  }

  .tabs {
    display: flex;
    border-bottom: 1px solid #333;
    padding: 0 8px;
  }

  .tab-btn {
    padding: 8px 16px;
    background: none;
    border: none;
    border-bottom: 2px solid transparent;
    color: #888;
    cursor: pointer;
    font-size: 13px;
    transition: color 0.15s, border-color 0.15s;
  }

  .tab-btn:hover {
    color: #ccc;
  }

  .tab-btn.active {
    color: #4A90D9;
    border-bottom-color: #4A90D9;
  }

  .tab-body {
    padding: 16px;
    overflow-y: auto;
    flex: 1;
    min-height: 300px;
  }

  .dialog-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 16px;
    border-top: 1px solid #333;
  }

  .reset-btn {
    padding: 6px 14px;
    background: none;
    border: 1px solid #555;
    border-radius: 4px;
    color: #888;
    cursor: pointer;
    font-size: 13px;
  }

  .reset-btn:hover {
    background: #333;
    color: #ccc;
  }

  .close-footer-btn {
    padding: 6px 20px;
    background: #4A90D9;
    border: none;
    border-radius: 4px;
    color: #fff;
    cursor: pointer;
    font-size: 13px;
  }

  .close-footer-btn:hover {
    background: #5AA0E9;
  }
</style>
