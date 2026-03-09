<script lang="ts">
  import { openInFinder, moveToTrash, permanentDelete, showGetInfo, openInTerminal, openFile } from "../api";
  import { confirm } from "@tauri-apps/plugin-dialog";
  import { tree, refreshSubtree, partialScanning } from "../stores/scanStore";
  import { isSpecialPath } from "../utils/specialNodes";
  import { cleanupActions, runCleanup } from "../stores/cleanupStore";
  import type { CleanupAction } from "../types";

  interface Props {
    x: number;
    y: number;
    path: string;
    name: string;
    isDir: boolean;
    size: number;
    childCount: number;
    onClose: () => void;
  }

  let { x, y, path, name, isDir, size, childCount, onClose }: Props = $props();

  let isSpecial = $derived(isSpecialPath(path));

  function formatSize(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
  }

  async function handleOpen() {
    try {
      await openFile(path);
    } catch (e) {
      console.error("Failed to open file:", e);
    }
    onClose();
  }

  async function handleRevealInFinder() {
    try {
      await openInFinder(path);
    } catch (e) {
      console.error("Failed to open in Finder:", e);
    }
    onClose();
  }

  async function handleOpenInTerminal() {
    try {
      await openInTerminal(path);
    } catch (e) {
      console.error("Failed to open in Terminal:", e);
    }
    onClose();
  }

  async function handleCopyPath() {
    try {
      await navigator.clipboard.writeText(path);
    } catch (_) {}
    onClose();
  }

  async function handleGetInfo() {
    try {
      await showGetInfo(path);
    } catch (e) {
      console.error("Failed to show Get Info:", e);
    }
    onClose();
  }

  async function handleMoveToTrash() {
    const ok = window.confirm(`Move "${name}" to Trash?`);
    if (!ok) {
      onClose();
      return;
    }
    try {
      await moveToTrash(path);
      tree.update((t) => {
        if (!t) return t;
        removeNode(t, path);
        return { ...t };
      });
    } catch (e) {
      console.error("Failed to move to trash:", e);
      window.alert(`Failed to move to trash: ${e}`);
    }
    onClose();
  }

  async function handlePermanentDelete() {
    // First confirmation
    let message: string;
    if (isDir) {
      message = `Permanently delete "${name}"?\n\nThis folder contains ${childCount.toLocaleString()} items (${formatSize(size)}).\n\nThis action CANNOT be undone.`;
    } else {
      message = `Permanently delete "${name}" (${formatSize(size)})?\n\nThis action CANNOT be undone.`;
    }

    const firstConfirm = await confirm(message, {
      title: "Delete Permanently",
      kind: "warning",
      okLabel: "Delete",
      cancelLabel: "Cancel",
    });
    if (!firstConfirm) {
      onClose();
      return;
    }

    // Second confirmation for safety
    const secondConfirm = await confirm(
      `Are you absolutely sure?\n\n"${name}" will be permanently deleted and cannot be recovered.`,
      {
        title: "Final Confirmation",
        kind: "warning",
        okLabel: "Delete Permanently",
        cancelLabel: "Cancel",
      }
    );
    if (!secondConfirm) {
      onClose();
      return;
    }

    try {
      await permanentDelete(path);
      tree.update((t) => {
        if (!t) return t;
        removeNode(t, path);
        return { ...t };
      });
    } catch (e) {
      console.error("Failed to permanently delete:", e);
      window.alert(`Failed to delete: ${e}`);
    }
    onClose();
  }

  async function handleRefresh() {
    onClose();
    await refreshSubtree(path);
  }

  let filteredCleanups = $derived(
    $cleanupActions.filter((a: CleanupAction) => {
      if (!a.enabled || !a.name.trim() || !a.command.trim()) return false;
      if (a.target === "Files" && isDir) return false;
      if (a.target === "Dirs" && !isDir) return false;
      return true;
    })
  );

  async function handleCleanup(action: CleanupAction) {
    if (action.confirm) {
      const ok = await confirm(`Run "${action.name}" on "${name}"?`, {
        title: "Run Cleanup",
        kind: "info",
        okLabel: "Run",
        cancelLabel: "Cancel",
      });
      if (!ok) {
        onClose();
        return;
      }
    }

    try {
      const result = await runCleanup(action.id, path, name);
      if (!action.run_in_terminal && result) {
        console.log(`Cleanup "${action.name}" result:`, result);
      }
      if (action.refresh_after && isDir) {
        await refreshSubtree(path);
      }
    } catch (e) {
      console.error(`Cleanup "${action.name}" failed:`, e);
      window.alert(`Cleanup failed: ${e}`);
    }
    onClose();
  }

  function removeNode(node: { children: { path: string; children: any[] }[] }, targetPath: string) {
    const idx = node.children.findIndex((c) => c.path === targetPath);
    if (idx >= 0) {
      node.children.splice(idx, 1);
      return true;
    }
    for (const child of node.children) {
      if (child.children && removeNode(child, targetPath)) return true;
    }
    return false;
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<!-- svelte-ignore a11y_click_events_have_key_events -->
<div class="overlay" onclick={onClose} oncontextmenu={(e) => { e.preventDefault(); onClose(); }}>
  <div class="context-menu" style="left: {x}px; top: {y}px" onclick={(e) => e.stopPropagation()}>
    <button class="menu-item" onclick={handleOpen} disabled={isSpecial}>
      Open
    </button>
    <button class="menu-item" onclick={handleRevealInFinder} disabled={isSpecial}>
      Reveal in Finder
    </button>
    <button class="menu-item" onclick={handleOpenInTerminal} disabled={isSpecial}>
      Open in Terminal
    </button>
    <div class="separator"></div>
    <button class="menu-item" onclick={handleCopyPath} disabled={isSpecial}>
      Copy Path
    </button>
    <button class="menu-item" onclick={handleGetInfo} disabled={isSpecial}>
      Get Info
    </button>
    {#if isDir}
      <div class="separator"></div>
      <button class="menu-item" onclick={handleRefresh} disabled={$partialScanning || isSpecial}>
        Refresh
      </button>
    {/if}
    {#if filteredCleanups.length > 0 && !isSpecial}
      <div class="separator"></div>
      {#each filteredCleanups as action}
        <button class="menu-item" onclick={() => handleCleanup(action)}>
          <span class="cleanup-name">{action.name}</span>
          <span class="cleanup-shortcut">Cmd+Shift+{action.id}</span>
        </button>
      {/each}
    {/if}
    {#if !isSpecial}
      <div class="separator"></div>
      <button class="menu-item danger" onclick={handleMoveToTrash}>
        Move to Trash
      </button>
      <button class="menu-item danger" onclick={handlePermanentDelete}>
        Delete Permanently
      </button>
    {/if}
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    z-index: 200;
  }

  .context-menu {
    position: fixed;
    background: #2d2d2d;
    border: 1px solid #555;
    border-radius: 6px;
    padding: 4px 0;
    min-width: 180px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.5);
    z-index: 201;
  }

  .menu-item {
    display: block;
    width: 100%;
    padding: 6px 16px;
    background: none;
    border: none;
    color: #ccc;
    font-size: 13px;
    text-align: left;
    cursor: pointer;
  }

  .menu-item:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .menu-item:hover:not(:disabled) {
    background: #3a3a3a;
    color: #fff;
  }

  .menu-item.danger {
    color: #e55;
  }

  .menu-item.danger:hover {
    background: #4a2020;
    color: #f88;
  }

  .separator {
    height: 1px;
    background: #444;
    margin: 4px 0;
  }

  .cleanup-name {
    flex: 1;
  }

  .cleanup-shortcut {
    font-size: 11px;
    color: #777;
    margin-left: 16px;
  }

  .menu-item:has(.cleanup-name) {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
</style>
