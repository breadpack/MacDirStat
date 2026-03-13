<script lang="ts">
  import { openInFinder, moveToTrash, permanentDelete, showGetInfo, openInTerminal, openFile, executeCleanupRecommendation } from "../api";
  import { confirm, message } from "@tauri-apps/plugin-dialog";
  import { tree, refreshSubtree, partialScanning } from "../stores/scanStore";
  import { isSpecialPath } from "../utils/specialNodes";
  import { cleanupActions, runCleanup } from "../stores/cleanupStore";
  import { removeNode } from "../utils/treeUtils";
  import type { CleanupAction } from "../types";
  import { formatSize } from "../utils/format";

  interface Props {
    x: number;
    y: number;
    path: string;
    name: string;
    isDir: boolean;
    size: number;
    childCount: number;
    cleanupPatternId?: string | null;
    onClose: () => void;
  }

  let { x, y, path, name, isDir, size, childCount, cleanupPatternId, onClose }: Props = $props();

  let isSpecial = $derived(isSpecialPath(path));


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
    const ok = await confirm(`Move "${name}" to Trash?`);
    if (!ok) {
      onClose();
      return;
    }
    try {
      await moveToTrash(path);
      tree.update((t) => {
        if (!t) return t;
        return removeNode(t, path) ?? t;
      });
    } catch (e) {
      console.error("Failed to move to trash:", e);
      await message(`Failed to move to trash: ${e}`, { kind: "error" });
    }
    onClose();
  }

  async function handlePermanentDelete() {
    // First confirmation
    let confirmMsg: string;
    if (isDir) {
      confirmMsg = `Permanently delete "${name}"?\n\nThis folder contains ${childCount.toLocaleString()} items (${formatSize(size)}).\n\nThis action CANNOT be undone.`;
    } else {
      confirmMsg = `Permanently delete "${name}" (${formatSize(size)})?\n\nThis action CANNOT be undone.`;
    }

    const firstConfirm = await confirm(confirmMsg, {
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
        return removeNode(t, path) ?? t;
      });
    } catch (e) {
      console.error("Failed to permanently delete:", e);
      await message(`Failed to delete: ${e}`, { kind: "error" });
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
      await message(`Cleanup failed: ${e}`, { kind: "error" });
    }
    onClose();
  }

<<<<<<< HEAD
  async function handleRecommendedCleanup() {
    if (!cleanupPatternId) return;
    const ok = await confirm(`Clean "${name}" (${formatSize(size)})?\n\nPattern: ${cleanupPatternId}`, {
      title: "Cleanup Recommendation",
      kind: "info",
      okLabel: "Clean",
      cancelLabel: "Cancel",
    });
    if (!ok) {
      onClose();
      return;
    }
    try {
      const result = await executeCleanupRecommendation(cleanupPatternId, [path]);
      if (result.success && isDir) {
        await refreshSubtree(path);
      }
    } catch (e) {
      console.error("Recommended cleanup failed:", e);
      await message(`Cleanup failed: ${e}`, { kind: "error" });
    }
    onClose();
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
    {#if cleanupPatternId && !isSpecial}
      <div class="separator"></div>
      <button class="menu-item cleanup-rec" onclick={handleRecommendedCleanup}>
        &#x1F9F9; Clean: {cleanupPatternId}
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

  .cleanup-rec {
    color: #6dba6d;
  }

  .cleanup-rec:hover {
    background: #1e3a1e;
    color: #8fdf8f;
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
