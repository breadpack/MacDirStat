<script lang="ts">
  import { scanning, progress } from "../stores/scanStore";
  import { formatNumber, formatSize } from "../utils/format";

  let isStructurePhase = $derived($progress.phase === "structure");

  let percent = $derived(
    $progress.total_dirs > 0 && !isStructurePhase
      ? ($progress.completed_dirs / $progress.total_dirs) * 100
      : 0
  );

  let statusText = $derived.by(() => {
    const p = $progress;
    if (isStructurePhase) {
      if (p.total_dirs > 0) {
        return `Scanning directory structure... (${formatNumber(p.total_dirs)} dirs found)`;
      }
      return "Scanning directory structure...";
    }
    if (p.current_dir_name) {
      return `Scanning ${p.current_dir_name} (${p.completed_dirs}/${p.total_dirs})`;
    }
    if (p.completed_dirs >= p.total_dirs && p.total_dirs > 0) {
      return `Finalizing... (${p.completed_dirs}/${p.total_dirs})`;
    }
    if (p.total_dirs > 0) {
      return `${p.completed_dirs}/${p.total_dirs} directories completed`;
    }
    return "Preparing...";
  });

  let statsText = $derived.by(() => {
    const p = $progress;
    if (isStructurePhase) return "";
    return `${formatNumber(p.files_scanned)} files \u00B7 ${formatSize(p.total_bytes)}`;
  });
</script>

{#if $scanning}
  <div class="progress-bar">
    <div class="progress-top">
      <span class="status">{statusText}</span>
      {#if statsText}
        <span class="stats">{statsText}</span>
      {/if}
    </div>

    <div class="bar-track">
      <div
        class="bar-fill"
        class:indeterminate={isStructurePhase}
        style="width: {isStructurePhase ? 100 : percent}%"
      ></div>
    </div>

    {#if $progress.current_path && !isStructurePhase}
      <div class="current-path" title={$progress.current_path}>
        {$progress.current_path}
      </div>
    {/if}
  </div>
{/if}

<style>
  .progress-bar {
    padding: 6px 12px;
    background: var(--bg-primary);
    border-bottom: 1px solid var(--border-color);
  }

  .progress-top {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 4px;
  }

  .status {
    font-size: 13px;
    color: var(--text-primary);
    font-weight: 500;
  }

  .stats {
    font-size: 12px;
    color: var(--text-secondary);
    font-variant-numeric: tabular-nums;
  }

  .bar-track {
    height: 6px;
    background: var(--border-color);
    border-radius: 3px;
    overflow: hidden;
  }

  .bar-fill {
    height: 100%;
    background: var(--accent-color);
    border-radius: 3px;
    transition: width 0.3s ease;
  }

  .bar-fill.indeterminate {
    animation: indeterminate 1.5s infinite;
    transform-origin: left;
  }

  .current-path {
    margin-top: 3px;
    font-size: 11px;
    color: var(--text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    direction: rtl;
    text-align: left;
  }

  @keyframes indeterminate {
    0% { transform: translateX(-100%); }
    100% { transform: translateX(100%); }
  }
</style>
