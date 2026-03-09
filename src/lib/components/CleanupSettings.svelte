<script lang="ts">
  import type { CleanupAction, CleanupTarget } from "../types";
  import { cleanupActions, saveCleanups, reloadCleanupActions } from "../stores/cleanupStore";

  interface Props {
    onClose: () => void;
  }

  let { onClose }: Props = $props();

  const DANGEROUS_PATTERNS = ["rm -rf", "sudo ", "mkfs", "dd if=", "> /dev/"];

  let editActions: CleanupAction[] = $state([]);
  let saving = $state(false);
  let error = $state("");

  // Initialize from store
  $effect(() => {
    const unsub = cleanupActions.subscribe((actions) => {
      // Deep clone to avoid mutating store directly
      editActions = JSON.parse(JSON.stringify(actions));
      // Pad to 10 slots
      while (editActions.length < 10) {
        editActions.push({
          id: editActions.length,
          name: "",
          command: "",
          enabled: false,
          target: "Both",
          confirm: false,
          run_in_terminal: false,
          refresh_after: false,
        });
      }
    });
    return unsub;
  });

  function isDangerous(command: string): boolean {
    const lower = command.toLowerCase();
    return DANGEROUS_PATTERNS.some((p) => lower.includes(p));
  }

  async function handleSave() {
    saving = true;
    error = "";
    try {
      // Only save actions that have a name and command, or reset empty ones
      const toSave = editActions.map((a, i) => ({
        ...a,
        id: i,
        enabled: a.enabled && a.name.trim() !== "" && a.command.trim() !== "",
      }));
      await saveCleanups(toSave);
      await reloadCleanupActions();
      onClose();
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }

  function handleCancel() {
    onClose();
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<!-- svelte-ignore a11y_click_events_have_key_events -->
<div class="modal-overlay" onclick={onClose}>
  <div class="modal" onclick={(e) => e.stopPropagation()}>
    <div class="modal-header">
      <h2>Cleanup Actions</h2>
      <button class="close-btn" onclick={onClose}>x</button>
    </div>

    <div class="help-bar">
      Variables: <code>%p</code> full path, <code>%n</code> file name,
      <code>%d</code> parent dir, <code>%e</code> extension
    </div>

    <div class="actions-list">
      {#each editActions as action, i}
        <div class="action-row" class:disabled={!action.enabled}>
          <div class="action-header">
            <label class="slot-label">
              <input type="checkbox" bind:checked={action.enabled} />
              <span class="slot-num">{i}</span>
            </label>
            <input
              type="text"
              class="name-input"
              placeholder="Action name"
              bind:value={action.name}
            />
            <span class="shortcut-hint">Cmd+Shift+{i}</span>
          </div>
          <div class="action-body">
            <input
              type="text"
              class="command-input"
              placeholder="Shell command (e.g. zip -r %p.zip %p)"
              bind:value={action.command}
            />
            {#if action.command && isDangerous(action.command)}
              <div class="warning">Warning: potentially dangerous command</div>
            {/if}
          </div>
          <div class="action-options">
            <label>
              Target:
              <select bind:value={action.target}>
                <option value="Both">Files & Dirs</option>
                <option value="Files">Files only</option>
                <option value="Dirs">Dirs only</option>
              </select>
            </label>
            <label>
              <input type="checkbox" bind:checked={action.confirm} />
              Confirm before run
            </label>
            <label>
              <input type="checkbox" bind:checked={action.run_in_terminal} />
              Run in Terminal
            </label>
            <label>
              <input type="checkbox" bind:checked={action.refresh_after} />
              Refresh after
            </label>
          </div>
        </div>
      {/each}
    </div>

    {#if error}
      <div class="error-msg">{error}</div>
    {/if}

    <div class="modal-footer">
      <button class="btn cancel" onclick={handleCancel}>Cancel</button>
      <button class="btn save" onclick={handleSave} disabled={saving}>
        {saving ? "Saving..." : "Save"}
      </button>
    </div>
  </div>
</div>

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
    width: 680px;
    max-height: 85vh;
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

  .close-btn {
    background: none;
    border: none;
    color: #888;
    font-size: 18px;
    cursor: pointer;
    padding: 2px 6px;
  }

  .close-btn:hover {
    color: #fff;
  }

  .help-bar {
    padding: 8px 16px;
    background: #333;
    font-size: 12px;
    color: #aaa;
    border-bottom: 1px solid #444;
  }

  .help-bar code {
    background: #444;
    padding: 1px 4px;
    border-radius: 3px;
    color: #ddd;
  }

  .actions-list {
    overflow-y: auto;
    flex: 1;
    padding: 8px 16px;
  }

  .action-row {
    border: 1px solid #444;
    border-radius: 6px;
    padding: 8px 10px;
    margin-bottom: 6px;
  }

  .action-row.disabled {
    opacity: 0.5;
  }

  .action-header {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 4px;
  }

  .slot-label {
    display: flex;
    align-items: center;
    gap: 4px;
    cursor: pointer;
  }

  .slot-num {
    font-size: 12px;
    color: #888;
    font-weight: 600;
    min-width: 12px;
  }

  .name-input {
    flex: 1;
    background: #333;
    border: 1px solid #555;
    border-radius: 4px;
    color: #ddd;
    padding: 4px 8px;
    font-size: 13px;
  }

  .shortcut-hint {
    font-size: 11px;
    color: #777;
    white-space: nowrap;
  }

  .action-body {
    margin-bottom: 4px;
  }

  .command-input {
    width: 100%;
    background: #333;
    border: 1px solid #555;
    border-radius: 4px;
    color: #ddd;
    padding: 4px 8px;
    font-size: 12px;
    font-family: monospace;
  }

  .warning {
    color: #e88;
    font-size: 11px;
    margin-top: 2px;
  }

  .action-options {
    display: flex;
    gap: 12px;
    flex-wrap: wrap;
    font-size: 12px;
    color: #aaa;
  }

  .action-options label {
    display: flex;
    align-items: center;
    gap: 4px;
    cursor: pointer;
  }

  .action-options select {
    background: #333;
    border: 1px solid #555;
    color: #ddd;
    border-radius: 3px;
    font-size: 11px;
    padding: 1px 4px;
  }

  .error-msg {
    color: #e55;
    padding: 4px 16px;
    font-size: 12px;
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 10px 16px;
    border-top: 1px solid #444;
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

  .btn.save {
    background: #2a7;
    color: #fff;
  }

  .btn.save:hover {
    background: #3b8;
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: default;
  }
</style>
