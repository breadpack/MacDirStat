<script lang="ts">
</script>

<div class="drop-overlay">
  <div class="drop-content">
    <div class="drop-icon">
      <svg width="64" height="64" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
        <path d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" />
        <path d="M12 11v6M9 14l3-3 3 3" />
      </svg>
    </div>
    <p class="drop-text">Drop folder to scan</p>
  </div>
</div>

<style>
  .drop-overlay {
    position: fixed;
    inset: 0;
    z-index: 300;
    background: rgba(74, 144, 217, 0.15);
    display: flex;
    align-items: center;
    justify-content: center;
    pointer-events: none;
  }

  .drop-content {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 16px;
    padding: 48px 64px;
    border: 3px dashed rgba(74, 144, 217, 0.6);
    border-radius: 16px;
    background: rgba(74, 144, 217, 0.1);
  }

  .drop-icon {
    color: rgba(74, 144, 217, 0.8);
  }

  .drop-text {
    margin: 0;
    font-size: 18px;
    font-weight: 500;
    color: rgba(74, 144, 217, 0.9);
  }
</style>
