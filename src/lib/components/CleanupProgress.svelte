<script lang="ts">
  import type { CleanupScanProgress } from "../types";

  interface Props {
    scanning: boolean;
    progress: CleanupScanProgress | null;
    executing: boolean;
    executingCurrent: string;
    executingDone: number;
    executingTotal: number;
    cancelRequested: boolean;
    onCancel: () => void;
  }

  let { scanning, progress, executing, executingCurrent, executingDone, executingTotal, cancelRequested, onCancel }: Props = $props();
</script>

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
        onclick={onCancel}
        disabled={cancelRequested}
      >
        {cancelRequested ? "Cancelling..." : "Cancel"}
      </button>
    </div>
  </div>
{/if}

<style>
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
