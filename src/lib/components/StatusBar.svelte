<script lang="ts">
  import type { Snippet } from "svelte";
  import { tree } from "../stores/scanStore";
  import { selectedPath } from "../stores/selectionStore";
  import { formatSize, formatNumber } from "../utils/format";

  interface Props {
    logButton?: Snippet;
  }

  let { logButton }: Props = $props();

  let displayPath = $derived($selectedPath ?? "");
  let totalFiles = $derived($tree?.file_count ?? 0);
  let totalSize = $derived($tree?.size ?? 0);
</script>

<div class="status-bar">
  {#if logButton}
    {@render logButton()}
  {/if}
  <span class="path" title={displayPath}>{displayPath}</span>
  <span class="spacer"></span>
  {#if $tree}
    <span class="info">{formatNumber(totalFiles)} files</span>
    <span class="sep">|</span>
    <span class="info">{formatSize(totalSize)}</span>
  {/if}
</div>

<style>
  .status-bar {
    display: flex;
    align-items: center;
    padding: 4px 12px;
    background: #1e1e1e;
    border-top: 1px solid #333;
    font-size: 12px;
    color: #999;
    height: 24px;
  }

  .path {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 50%;
  }

  .spacer {
    flex: 1;
  }

  .info {
    flex-shrink: 0;
  }

  .sep {
    margin: 0 8px;
    color: #555;
  }
</style>
