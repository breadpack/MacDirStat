<script lang="ts">
  import type { CleanupRecommendation } from "../types";
  import { formatSize } from "../utils/format";
  import CleanupRecommendationItem from "./CleanupRecommendationItem.svelte";

  interface Props {
    category: string;
    items: CleanupRecommendation[];
    selectedIds: Set<string>;
    failedItems: Map<string, string>;
    onToggleItem: (patternId: string) => void;
    onCleanItem: (recommendation: CleanupRecommendation) => void;
    onCleanAll: () => void;
  }

  let { category, items, selectedIds, failedItems, onToggleItem, onCleanItem, onCleanAll }: Props = $props();

  let expanded = $state(true);

  let totalSize = $derived(items.reduce((sum, item) => sum + item.total_size, 0));
  let sortedItems = $derived([...items].sort((a, b) => b.total_size - a.total_size));
</script>

<div class="category-group">
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div class="category-header" onclick={() => (expanded = !expanded)}>
    <span class="toggle">{expanded ? "▾" : "▸"}</span>
    <span class="category-name">{category}</span>
    <span class="category-size">{formatSize(totalSize)}</span>
    <span class="item-count">{items.length} item{items.length !== 1 ? "s" : ""}</span>
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <button
      class="clean-all-btn"
      onclick={(e) => {
        e.stopPropagation();
        onCleanAll();
      }}
    >
      Clean All
    </button>
  </div>

  {#if expanded}
    <div class="category-items">
      {#each sortedItems as item (item.pattern_id)}
        <CleanupRecommendationItem
          recommendation={item}
          selected={selectedIds.has(item.pattern_id)}
          errorMessage={failedItems.get(item.pattern_id) ?? null}
          onToggle={() => onToggleItem(item.pattern_id)}
          onClean={() => onCleanItem(item)}
        />
      {/each}
    </div>
  {/if}
</div>

<style>
  .category-group {
    margin-bottom: 4px;
  }

  .category-header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 10px;
    background: #333;
    border-radius: 4px;
    cursor: pointer;
    user-select: none;
  }

  .category-header:hover {
    background: #3a3a3a;
  }

  .toggle {
    font-size: 11px;
    color: #888;
    width: 12px;
    flex-shrink: 0;
  }

  .category-name {
    font-size: 13px;
    font-weight: 600;
    color: #ddd;
    flex: 1;
  }

  .category-size {
    font-size: 12px;
    color: #4a90d9;
    font-weight: 600;
    flex-shrink: 0;
  }

  .item-count {
    font-size: 11px;
    color: #888;
    flex-shrink: 0;
  }

  .clean-all-btn {
    background: #444;
    border: 1px solid #555;
    border-radius: 3px;
    color: #ccc;
    font-size: 11px;
    padding: 2px 8px;
    cursor: pointer;
    flex-shrink: 0;
  }

  .clean-all-btn:hover {
    background: #555;
    color: #fff;
  }

  .category-items {
    padding-left: 12px;
  }
</style>
