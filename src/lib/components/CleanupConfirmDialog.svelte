<script lang="ts">
  import type { RiskLevel } from "../types";

  interface Props {
    title: string;
    message: string;
    riskLevel: RiskLevel;
    onConfirm: () => void;
    onCancel: () => void;
  }

  let { title, message, riskLevel, onConfirm, onCancel }: Props = $props();

  let borderColor = $derived(
    riskLevel === "Safe" ? "#4caf50" : riskLevel === "Caution" ? "#ff9800" : "#f44336",
  );
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<!-- svelte-ignore a11y_click_events_have_key_events -->
<div class="dialog-overlay" onclick={onCancel}>
  <div class="dialog" style="border-color: {borderColor}" onclick={(e) => e.stopPropagation()}>
    <h3 class="dialog-title">{title}</h3>
    <p class="dialog-message">{message}</p>
    <div class="dialog-actions">
      <button class="btn cancel" onclick={onCancel}>Cancel</button>
      <button class="btn confirm" onclick={onConfirm}>Confirm</button>
    </div>
  </div>
</div>

<style>
  .dialog-overlay {
    position: fixed;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    background: rgba(0, 0, 0, 0.7);
    z-index: 400;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .dialog {
    background: #2a2a2a;
    border: 2px solid #555;
    border-radius: 8px;
    padding: 20px 24px;
    max-width: 420px;
    width: 90%;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.6);
  }

  .dialog-title {
    margin: 0 0 12px 0;
    font-size: 15px;
    color: #eee;
  }

  .dialog-message {
    margin: 0 0 20px 0;
    font-size: 13px;
    color: #bbb;
    line-height: 1.5;
  }

  .dialog-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  .btn {
    padding: 6px 16px;
    border: none;
    border-radius: 4px;
    font-size: 13px;
    cursor: pointer;
  }

  .btn.cancel {
    background: #444;
    color: #ccc;
  }

  .btn.cancel:hover {
    background: #555;
  }

  .btn.confirm {
    background: #4a90d9;
    color: #fff;
  }

  .btn.confirm:hover {
    background: #5aa0e9;
  }
</style>
