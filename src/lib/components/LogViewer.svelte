<script lang="ts">
  import { scanLogs } from "../stores/scanStore";
  import type { ScanLogEntry } from "../types";

  interface Props {
    onClose: () => void;
  }

  let { onClose }: Props = $props();

  let filterLevel = $state<string>("all");
  let autoScroll = $state(true);
  let listEl: HTMLDivElement | undefined = $state();

  let filteredLogs = $derived(
    filterLevel === "all"
      ? $scanLogs
      : $scanLogs.filter((l) => l.level === filterLevel)
  );

  let counts = $derived.by(() => {
    const c = { error: 0, warn: 0, info: 0, skip: 0 };
    for (const log of $scanLogs) {
      c[log.level as keyof typeof c]++;
    }
    return c;
  });

  $effect(() => {
    // trigger on filteredLogs change
    filteredLogs;
    if (autoScroll && listEl) {
      requestAnimationFrame(() => {
        listEl!.scrollTop = listEl!.scrollHeight;
      });
    }
  });

  function levelColor(level: string): string {
    switch (level) {
      case "error": return "#e55";
      case "warn": return "#da4";
      case "skip": return "#888";
      case "info": return "#5a5";
      default: return "#888";
    }
  }

  function levelIcon(level: string): string {
    switch (level) {
      case "error": return "\u2718";
      case "warn": return "\u26A0";
      case "skip": return "\u21B7";
      case "info": return "\u2139";
      default: return "\u00B7";
    }
  }
</script>

<div class="log-viewer">
  <div class="log-header">
    <span class="log-title">Scan Log</span>
    <div class="log-filters">
      <button
        class="filter-btn"
        class:active={filterLevel === "all"}
        onclick={() => filterLevel = "all"}
      >
        All ({$scanLogs.length})
      </button>
      <button
        class="filter-btn error"
        class:active={filterLevel === "error"}
        onclick={() => filterLevel = "error"}
      >
        Errors ({counts.error})
      </button>
      <button
        class="filter-btn warn"
        class:active={filterLevel === "warn"}
        onclick={() => filterLevel = "warn"}
      >
        Warnings ({counts.warn})
      </button>
      <button
        class="filter-btn skip"
        class:active={filterLevel === "skip"}
        onclick={() => filterLevel = "skip"}
      >
        Skipped ({counts.skip})
      </button>
    </div>
    <label class="auto-scroll">
      <input type="checkbox" bind:checked={autoScroll} />
      Auto-scroll
    </label>
    <button class="close-btn" onclick={onClose}>\u2715</button>
  </div>

  <div class="log-list" bind:this={listEl}>
    {#each filteredLogs as entry (entry)}
      <div class="log-entry">
        <span class="log-icon" style="color: {levelColor(entry.level)}">
          {levelIcon(entry.level)}
        </span>
        <span class="log-level" style="color: {levelColor(entry.level)}">
          {entry.level.toUpperCase()}
        </span>
        <span class="log-message">{entry.message}</span>
        <span class="log-path" title={entry.path}>{entry.path}</span>
      </div>
    {/each}
    {#if filteredLogs.length === 0}
      <div class="log-empty">No log entries</div>
    {/if}
  </div>
</div>

<style>
  .log-viewer {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bg-primary);
    border-top: 1px solid var(--border-color);
  }

  .log-header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 12px;
    background: var(--bg-header);
    border-bottom: 1px solid var(--border-color);
    flex-shrink: 0;
  }

  .log-title {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
    margin-right: 8px;
  }

  .log-filters {
    display: flex;
    gap: 4px;
  }

  .filter-btn {
    background: var(--bg-secondary);
    border: 1px solid var(--border-color-strong);
    border-radius: 3px;
    color: var(--text-secondary);
    font-size: 11px;
    padding: 2px 8px;
    cursor: pointer;
  }

  .filter-btn:hover { background: var(--bg-tertiary); }
  .filter-btn.active { background: var(--hover-bg); color: var(--text-heading); border-color: var(--border-color-strong); }
  .filter-btn.error.active { color: #e55; }
  .filter-btn.warn.active { color: #da4; }
  .filter-btn.skip.active { color: #888; }

  .auto-scroll {
    margin-left: auto;
    font-size: 11px;
    color: var(--text-secondary);
    display: flex;
    align-items: center;
    gap: 4px;
    cursor: pointer;
  }

  .close-btn {
    background: none;
    border: none;
    color: var(--text-secondary);
    font-size: 16px;
    cursor: pointer;
    padding: 0 4px;
  }
  .close-btn:hover { color: var(--text-heading); }

  .log-list {
    flex: 1;
    overflow-y: auto;
    font-family: "SF Mono", "Menlo", "Monaco", monospace;
    font-size: 12px;
  }

  .log-entry {
    display: flex;
    align-items: baseline;
    gap: 6px;
    padding: 2px 12px;
    border-bottom: 1px solid var(--bg-secondary);
  }

  .log-entry:hover {
    background: var(--bg-header);
  }

  .log-icon {
    flex-shrink: 0;
    width: 14px;
    text-align: center;
  }

  .log-level {
    flex-shrink: 0;
    width: 44px;
    font-size: 10px;
    font-weight: 600;
  }

  .log-message {
    color: var(--text-primary);
    white-space: nowrap;
    flex-shrink: 0;
  }

  .log-path {
    color: var(--text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    direction: rtl;
    text-align: left;
    min-width: 0;
  }

  .log-empty {
    padding: 16px;
    text-align: center;
    color: var(--text-secondary);
  }
</style>
