<script lang="ts">
  import { checkFullDiskAccess, openFullDiskAccessSettings } from "../api";

  interface Props {
    onGranted: () => void;
  }

  let { onGranted }: Props = $props();

  let checking = $state(false);

  async function openSettings() {
    await openFullDiskAccessSettings();
  }

  async function recheck() {
    checking = true;
    try {
      const granted = await checkFullDiskAccess();
      if (granted) {
        onGranted();
      }
    } finally {
      checking = false;
    }
  }

  async function skipAnyway() {
    onGranted();
  }
</script>

<div class="overlay" role="dialog" aria-modal="true" aria-labelledby="fda-title">
  <div class="dialog">
    <div class="icon">&#x1F512;</div>
    <h2 id="fda-title">Full Disk Access Required</h2>
    <p class="description">
      MacDirStat needs <strong>Full Disk Access</strong> permission to scan all directories on your disk.
      Without this permission, some protected folders will be skipped.
    </p>

    <div class="steps">
      <div class="step">
        <span class="step-number">1</span>
        <span>Click <strong>Open System Settings</strong> below</span>
      </div>
      <div class="step">
        <span class="step-number">2</span>
        <span>Enable the toggle next to <strong>MacDirStat</strong></span>
      </div>
      <div class="step">
        <span class="step-number">3</span>
        <span>Come back and click <strong>I've Granted Access</strong></span>
      </div>
    </div>

    <div class="actions">
      <button class="primary" onclick={openSettings}>
        Open System Settings
      </button>
      <button class="secondary" onclick={recheck} disabled={checking}>
        {checking ? "Checking..." : "I've Granted Access"}
      </button>
    </div>

    <button class="skip" onclick={skipAnyway}>
      Continue without Full Disk Access
    </button>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: var(--overlay-bg);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .dialog {
    background: var(--bg-secondary);
    border: 1px solid var(--border-color-strong);
    border-radius: 12px;
    padding: 32px 40px;
    max-width: 480px;
    width: 90%;
    text-align: center;
  }

  .icon {
    font-size: 48px;
    margin-bottom: 8px;
  }

  h2 {
    margin: 0 0 12px;
    font-size: 20px;
    font-weight: 600;
    color: var(--text-heading);
  }

  .description {
    color: var(--text-secondary);
    font-size: 14px;
    line-height: 1.5;
    margin: 0 0 24px;
  }

  .steps {
    text-align: left;
    margin-bottom: 24px;
  }

  .step {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 0;
    font-size: 14px;
    color: var(--text-primary);
  }

  .step-number {
    width: 24px;
    height: 24px;
    border-radius: 50%;
    background: var(--accent-color);
    color: #fff;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 13px;
    font-weight: 600;
    flex-shrink: 0;
  }

  .actions {
    display: flex;
    gap: 12px;
    margin-bottom: 16px;
  }

  button {
    border: none;
    border-radius: 8px;
    padding: 10px 20px;
    font-size: 14px;
    cursor: pointer;
    flex: 1;
  }

  .primary {
    background: var(--accent-color);
    color: #fff;
  }

  .primary:hover {
    background: color-mix(in srgb, var(--accent-color), #fff 15%);
  }

  .secondary {
    background: var(--bg-tertiary);
    color: var(--text-primary);
    border: 1px solid var(--border-color-strong);
  }

  .secondary:hover:not(:disabled) {
    background: var(--hover-bg);
  }

  .secondary:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .skip {
    background: none;
    color: var(--text-secondary);
    font-size: 12px;
    text-decoration: underline;
    padding: 4px;
    flex: none;
  }

  .skip:hover {
    color: var(--text-primary);
  }
</style>
