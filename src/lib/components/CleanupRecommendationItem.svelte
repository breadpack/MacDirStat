<script lang="ts">
  import type { CleanupRecommendation } from "../types";
  import { formatSize } from "../utils/format";

  interface Props {
    recommendation: CleanupRecommendation;
    selected: boolean;
    errorMessage: string | null;
    onToggle: () => void;
    onClean: () => void;
  }

  let { recommendation, selected, errorMessage, onToggle, onClean }: Props = $props();

  let expanded = $state(false);
  let isWarning = $derived(recommendation.risk_level === "Warning");

  let riskColor = $derived(
    recommendation.risk_level === "Safe"
      ? "#4caf50"
      : recommendation.risk_level === "Caution"
        ? "#ff9800"
        : "#f44336",
  );
</script>

<div class="item" class:item-failed={errorMessage} title={recommendation.description}>
  <div class="item-main">
    <label class="checkbox-area">
      <input
        type="checkbox"
        checked={selected}
        disabled={isWarning}
        onchange={onToggle}
      />
    </label>

    <span class="risk-indicator" style="color: {riskColor}">
      {#if recommendation.risk_level === "Safe"}
        <span class="dot">&#9679;</span>
      {:else if recommendation.risk_level === "Caution"}
        <span class="triangle">&#9650;</span>
      {:else}
        <span class="circle">&#9673;</span>
      {/if}
    </span>

    <span class="pattern-name">{recommendation.pattern_name}</span>
    <span class="risk-label" style="color: {riskColor}">{recommendation.risk_level}</span>
    <span class="size">{formatSize(recommendation.total_size)}</span>

    {#if recommendation.paths.length > 1}
      <button
        class="expand-btn"
        onclick={() => (expanded = !expanded)}
        title="Show locations"
      >
        {expanded ? "▾" : "▸"} {recommendation.paths.length} locations
      </button>
    {/if}

    <button class="clean-btn" onclick={onClean}>Clean</button>
  </div>

  {#if errorMessage}
    <div class="error-line">Failed: {errorMessage}</div>
  {:else}
    <div class="description">{recommendation.description}</div>
  {/if}

  {#if expanded && recommendation.paths.length > 1}
    <ul class="paths-list">
      {#each recommendation.paths as path}
        <li class="path-item">{path}</li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .item {
    padding: 6px 8px;
    border-bottom: 1px solid #333;
  }

  .item:last-child {
    border-bottom: none;
  }

  .item-main {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .checkbox-area {
    display: flex;
    align-items: center;
    cursor: pointer;
  }

  .risk-indicator {
    font-size: 10px;
    width: 14px;
    text-align: center;
    flex-shrink: 0;
  }

  .dot {
    font-size: 12px;
  }

  .triangle {
    font-size: 10px;
  }

  .circle {
    font-size: 12px;
  }

  .pattern-name {
    flex: 1;
    font-size: 13px;
    color: #ddd;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .risk-label {
    font-size: 11px;
    flex-shrink: 0;
  }

  .size {
    font-size: 12px;
    color: #aaa;
    font-variant-numeric: tabular-nums;
    min-width: 60px;
    text-align: right;
    flex-shrink: 0;
  }

  .expand-btn {
    background: none;
    border: none;
    color: #888;
    font-size: 11px;
    cursor: pointer;
    padding: 2px 4px;
    white-space: nowrap;
    flex-shrink: 0;
  }

  .expand-btn:hover {
    color: #ccc;
  }

  .clean-btn {
    background: #444;
    border: 1px solid #555;
    border-radius: 3px;
    color: #ccc;
    font-size: 11px;
    padding: 2px 8px;
    cursor: pointer;
    flex-shrink: 0;
  }

  .clean-btn:hover {
    background: #555;
    color: #fff;
  }

  .item-failed {
    background: rgba(244, 67, 54, 0.08);
  }

  .description {
    font-size: 11px;
    color: #888;
    margin: 2px 0 0 30px;
  }

  .error-line {
    font-size: 11px;
    color: #f44336;
    margin: 2px 0 0 30px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .paths-list {
    margin: 4px 0 0 30px;
    padding: 0;
    list-style: none;
  }

  .path-item {
    font-size: 11px;
    color: #777;
    font-family: monospace;
    padding: 1px 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
