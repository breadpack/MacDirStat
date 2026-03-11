<script lang="ts">
  import { Channel } from "@tauri-apps/api/core";
  import { untrack } from "svelte";
  import type { CleanupRecommendation, CleanupScanProgress, CleanupCategory } from "../types";
  import { scanCleanupRecommendations, executeCleanupRecommendation } from "../api";
  import {
    recommendations,
    cleanupScanning,
    cleanupProgress,
    selectedPatternIds,
    removeRecommendation,
    resetCleanupRecommendations,
  } from "../stores/cleanupRecommendationStore";
  import { tree } from "../stores/scanStore";
  import { refreshSubtree } from "../stores/scanStore";
  import { removeNode, findNode } from "../utils/treeUtils";
  import { formatSize } from "../utils/format";
  import CleanupCategoryGroup from "./CleanupCategoryGroup.svelte";
  import CleanupConfirmDialog from "./CleanupConfirmDialog.svelte";

  const CATEGORY_NAMES: Record<string, string> = {
    DevTools: "Developer Tools",
    PackageManager: "Package Managers",
    Container: "Containers",
    Browser: "Browsers",
    IDE: "IDEs",
    System: "System",
    CloudStorage: "Cloud Storage",
    AppData: "App Data",
    Media: "Media",
  };

  interface Props {
    onClose: () => void;
  }

  let { onClose }: Props = $props();

  let recs: CleanupRecommendation[] = $state([]);
  let scanning = $state(false);
  let progress: CleanupScanProgress | null = $state(null);
  let selected: Set<string> = $state(new Set());
  let error = $state("");
  let executing = $state(false);
  let executingCurrent = $state("");
  let executingDone = $state(0);
  let executingTotal = $state(0);
  let cancelRequested = $state(false);
  let failedItems: Map<string, string> = $state(new Map()); // pattern_id -> error message

  // Confirmation dialog state
  let confirmDialog: {
    title: string;
    message: string;
    riskLevel: "Safe" | "Caution" | "Warning";
    onConfirm: () => void;
  } | null = $state(null);

  // Subscribe to stores
  $effect(() => {
    const unsubs = [
      recommendations.subscribe((v) => (recs = v)),
      cleanupScanning.subscribe((v) => (scanning = v)),
      cleanupProgress.subscribe((v) => (progress = v)),
      selectedPatternIds.subscribe((v) => (selected = v)),
    ];
    return () => unsubs.forEach((u) => u());
  });

  // Categories grouped and sorted by total size descending
  let categoryGroups = $derived.by(() => {
    const groups = new Map<CleanupCategory, CleanupRecommendation[]>();
    for (const rec of recs) {
      const existing = groups.get(rec.category) ?? [];
      existing.push(rec);
      groups.set(rec.category, existing);
    }
    return [...groups.entries()]
      .map(([cat, items]) => ({
        category: cat,
        displayName: CATEGORY_NAMES[cat] ?? cat,
        items,
        totalSize: items.reduce((s, i) => s + i.total_size, 0),
      }))
      .sort((a, b) => b.totalSize - a.totalSize);
  });

  let totalReclaimable = $derived(recs.reduce((s, r) => s + r.total_size, 0));
  let selectedSize = $derived(
    recs.filter((r) => selected.has(r.pattern_id)).reduce((s, r) => s + r.total_size, 0),
  );

  // Auto-scan on mount (untrack to prevent re-runs when state changes)
  $effect(() => {
    untrack(() => startScan());
  });

  async function startScan() {
    if (scanning) return;
    resetCleanupRecommendations();
    cleanupScanning.set(true);
    error = "";
    failedItems = new Map();

    try {
      const channel = new Channel<CleanupScanProgress>();
      channel.onmessage = (p) => {
        cleanupProgress.set(p);
      };

      const results = await scanCleanupRecommendations(channel);
      recommendations.set(results);

      // Default selection: Safe checked, Caution unchecked, Warning disabled
      const defaultSelected = new Set<string>();
      for (const r of results) {
        if (r.risk_level === "Safe") {
          defaultSelected.add(r.pattern_id);
        }
      }
      selectedPatternIds.set(defaultSelected);
    } catch (e) {
      error = String(e);
    } finally {
      cleanupScanning.set(false);
      cleanupProgress.set(null);
    }
  }

  function toggleItem(patternId: string) {
    selectedPatternIds.update((ids) => {
      const next = new Set(ids);
      if (next.has(patternId)) {
        next.delete(patternId);
      } else {
        next.add(patternId);
      }
      return next;
    });
  }

  async function cleanItem(rec: CleanupRecommendation) {
    if (rec.risk_level === "Caution" || rec.risk_level === "Warning") {
      confirmDialog = {
        title: `Clean ${rec.pattern_name}?`,
        message: `This will clean ${formatSize(rec.total_size)} from ${rec.paths.length} location${rec.paths.length !== 1 ? "s" : ""}. Risk level: ${rec.risk_level}. ${rec.description}`,
        riskLevel: rec.risk_level,
        onConfirm: () => {
          confirmDialog = null;
          doClean(rec);
        },
      };
    } else {
      await doClean(rec);
    }
  }

  async function doClean(rec: CleanupRecommendation): Promise<boolean> {
    try {
      await executeCleanupRecommendation(rec.pattern_id, rec.paths, rec.cleanup_method);
      removeRecommendation(rec.pattern_id);
      // Clear failure if previously failed
      failedItems.delete(rec.pattern_id);
      failedItems = new Map(failedItems);

      // Update tree to reflect freed space
      updateTreeAfterCleanup(rec);

      return true;
    } catch (e) {
      const msg = String(e);
      failedItems.set(rec.pattern_id, msg);
      failedItems = new Map(failedItems);
      return false;
    }
  }

  function updateTreeAfterCleanup(rec: CleanupRecommendation) {
    if (rec.paths.length === 0) return;

    const method = rec.cleanup_method;

    if (method.type === "Delete") {
      // Instantly remove nodes from tree
      tree.update((t) => {
        if (!t) return t;
        let updated = t;
        for (const p of rec.paths) {
          updated = removeNode(updated, p);
        }
        return updated;
      });
    } else if (method.type === "ShellCommand") {
      // Re-scan affected paths' parents for accurate sizes
      for (const p of rec.paths) {
        const parentPath = p.substring(0, p.lastIndexOf("/")) || "/";
        const currentTree = getTreeSnapshot();
        if (currentTree && findNode(currentTree, parentPath)) {
          refreshSubtree(parentPath);
        }
      }
    }
  }

  function getTreeSnapshot(): import("../types").FileNode | null {
    let snapshot: import("../types").FileNode | null = null;
    tree.subscribe((t) => (snapshot = t))();
    return snapshot;
  }

  async function runBatch(items: CleanupRecommendation[]) {
    executing = true;
    cancelRequested = false;
    executingDone = 0;
    executingTotal = items.length;
    error = "";

    try {
      for (const rec of items) {
        if (cancelRequested) break;
        executingCurrent = rec.pattern_name;
        await doClean(rec);
        executingDone++;
      }
    } finally {
      executing = false;
      executingCurrent = "";
      cancelRequested = false;
    }
  }

  async function cleanAllInCategory(items: CleanupRecommendation[]) {
    const safeItems = items.filter((i) => i.risk_level === "Safe");
    const cautionItems = items.filter((i) => i.risk_level === "Caution");

    if (cautionItems.length > 0) {
      const batch = [...safeItems, ...cautionItems];
      const totalSize = batch.reduce((s, i) => s + i.total_size, 0);
      confirmDialog = {
        title: "Clean all items in category?",
        message: `This will clean ${formatSize(totalSize)} across ${batch.length} items (including ${cautionItems.length} Caution item${cautionItems.length !== 1 ? "s" : ""}).`,
        riskLevel: "Caution",
        onConfirm: () => {
          confirmDialog = null;
          runBatch(batch);
        },
      };
    } else {
      await runBatch(safeItems);
    }
  }

  async function cleanSelected() {
    const selectedRecs = recs.filter((r) => selected.has(r.pattern_id));
    if (selectedRecs.length === 0) return;

    const hasCaution = selectedRecs.some((r) => r.risk_level === "Caution");

    if (hasCaution) {
      confirmDialog = {
        title: "Clean selected items?",
        message: `This will clean ${formatSize(selectedSize)} across ${selectedRecs.length} items. Some items have Caution risk level.`,
        riskLevel: "Caution",
        onConfirm: () => {
          confirmDialog = null;
          runBatch(selectedRecs);
        },
      };
    } else {
      await runBatch(selectedRecs);
    }
  }
</script>

<svelte:window onkeydown={(e) => { if (e.key === "Escape" && !executing) onClose(); }} />

<!-- svelte-ignore a11y_no_static_element_interactions -->
<!-- svelte-ignore a11y_click_events_have_key_events -->
<div class="modal-overlay" onclick={() => { if (!executing) onClose(); }}>
  <div class="modal" class:disabled-overlay={executing} onclick={(e) => e.stopPropagation()}>
    <div class="modal-header">
      <h2>Cleanup Recommendations</h2>
      <div class="header-actions">
        <button class="scan-btn" onclick={startScan} disabled={scanning || executing}>
          {scanning ? "Scanning..." : "Scan"}
        </button>
        <button class="close-btn" onclick={onClose} disabled={executing}>&times;</button>
      </div>
    </div>

    {#if scanning && progress}
      <div class="progress-section">
        <div class="progress-bar-track">
          <div
            class="progress-bar-fill"
            style="width: {progress.total > 0 ? (progress.checked / progress.total) * 100 : 0}%"
          ></div>
        </div>
        <div class="progress-text">
          Checking: {progress.current_pattern} ({progress.checked}/{progress.total})
        </div>
      </div>
    {/if}

    {#if executing}
      <div class="executing-section">
        <div class="progress-bar-track">
          <div
            class="progress-bar-fill executing-fill"
            style="width: {executingTotal > 0 ? (executingDone / executingTotal) * 100 : 0}%"
          ></div>
        </div>
        <div class="executing-info">
          <span class="executing-text">
            Cleaning: {executingCurrent} ({executingDone}/{executingTotal})
          </span>
          <button
            class="cancel-btn"
            onclick={() => { cancelRequested = true; }}
            disabled={cancelRequested}
          >
            {cancelRequested ? "Cancelling..." : "Cancel"}
          </button>
        </div>
      </div>
    {/if}

    {#if error}
      <div class="error-msg">{error}</div>
    {/if}

    {#if !scanning && recs.length > 0}
      <div class="summary-line">
        {formatSize(totalReclaimable)} reclaimable across {recs.length} item{recs.length !== 1 ? "s" : ""}
      </div>
    {/if}

    <div class="content" class:content-disabled={executing}>
      {#if !scanning && recs.length === 0 && !error}
        <div class="empty-state">
          {progress === null && !error ? "Click Scan to find cleanup recommendations." : "No cleanup recommendations found."}
        </div>
      {/if}

      {#each categoryGroups as group (group.category)}
        <CleanupCategoryGroup
          category={group.displayName}
          items={group.items}
          selectedIds={selected}
          {failedItems}
          onToggleItem={toggleItem}
          onCleanItem={cleanItem}
          onCleanAll={() => cleanAllInCategory(group.items)}
        />
      {/each}
    </div>

    {#if !scanning && recs.length > 0}
      <div class="modal-footer">
        <span class="selected-info">
          Selected: {formatSize(selectedSize)}
        </span>
        <button
          class="btn primary"
          onclick={cleanSelected}
          disabled={selected.size === 0 || executing}
        >
          {executing ? "Cleaning..." : `Clean Selected (${selected.size})`}
        </button>
      </div>
    {/if}
  </div>
</div>

{#if confirmDialog}
  <CleanupConfirmDialog
    title={confirmDialog.title}
    message={confirmDialog.message}
    riskLevel={confirmDialog.riskLevel}
    onConfirm={confirmDialog.onConfirm}
    onCancel={() => (confirmDialog = null)}
  />
{/if}

<style>
  .modal-overlay {
    position: fixed;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    background: rgba(0, 0, 0, 0.6);
    z-index: 300;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .modal {
    background: #2a2a2a;
    border: 1px solid #555;
    border-radius: 8px;
    width: 700px;
    max-height: 80vh;
    display: flex;
    flex-direction: column;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.6);
  }

  .modal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 12px 16px;
    border-bottom: 1px solid #444;
  }

  .modal-header h2 {
    margin: 0;
    font-size: 16px;
    color: #eee;
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .scan-btn {
    background: #4a90d9;
    border: none;
    border-radius: 4px;
    color: #fff;
    font-size: 12px;
    padding: 4px 12px;
    cursor: pointer;
  }

  .scan-btn:hover:not(:disabled) {
    background: #5aa0e9;
  }

  .scan-btn:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .close-btn {
    background: none;
    border: none;
    color: #888;
    font-size: 20px;
    cursor: pointer;
    padding: 2px 6px;
    line-height: 1;
  }

  .close-btn:hover {
    color: #fff;
  }

  .progress-section {
    padding: 8px 16px;
    border-bottom: 1px solid #444;
  }

  .progress-bar-track {
    height: 4px;
    background: #444;
    border-radius: 2px;
    overflow: hidden;
    margin-bottom: 4px;
  }

  .progress-bar-fill {
    height: 100%;
    background: #4a90d9;
    border-radius: 2px;
    transition: width 0.2s ease;
  }

  .progress-text {
    font-size: 11px;
    color: #888;
  }

  .error-msg {
    color: #e55;
    padding: 8px 16px;
    font-size: 12px;
    border-bottom: 1px solid #444;
  }

  .summary-line {
    padding: 8px 16px;
    font-size: 13px;
    color: #4a90d9;
    font-weight: 600;
    border-bottom: 1px solid #444;
  }

  .content {
    flex: 1;
    overflow-y: auto;
    padding: 8px 12px;
  }

  .empty-state {
    text-align: center;
    padding: 40px 20px;
    color: #888;
    font-size: 13px;
  }

  .modal-footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 10px 16px;
    border-top: 1px solid #444;
  }

  .selected-info {
    font-size: 13px;
    color: #aaa;
  }

  .btn {
    padding: 6px 16px;
    border: none;
    border-radius: 4px;
    font-size: 13px;
    cursor: pointer;
  }

  .btn.primary {
    background: #4a90d9;
    color: #fff;
  }

  .btn.primary:hover:not(:disabled) {
    background: #5aa0e9;
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .disabled-overlay {
    pointer-events: auto;
  }

  .content-disabled {
    opacity: 0.4;
    pointer-events: none;
  }

  .executing-section {
    padding: 8px 16px;
    border-bottom: 1px solid #444;
    background: #333;
  }

  .executing-fill {
    background: #ff9800;
  }

  .executing-info {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-top: 4px;
  }

  .executing-text {
    font-size: 12px;
    color: #ff9800;
    font-weight: 500;
  }

  .cancel-btn {
    background: #555;
    border: 1px solid #666;
    border-radius: 4px;
    color: #eee;
    font-size: 11px;
    padding: 2px 10px;
    cursor: pointer;
  }

  .cancel-btn:hover:not(:disabled) {
    background: #e55;
    border-color: #e55;
    color: #fff;
  }

  .cancel-btn:disabled {
    opacity: 0.5;
    cursor: default;
  }
</style>
