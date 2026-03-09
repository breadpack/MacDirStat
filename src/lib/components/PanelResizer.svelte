<script lang="ts">
  interface Props {
    onResize: (delta: number) => void;
  }

  let { onResize }: Props = $props();

  let dragging = $state(false);
  let startX = 0;

  function onMouseDown(e: MouseEvent) {
    e.preventDefault();
    dragging = true;
    startX = e.clientX;

    const onMouseMove = (ev: MouseEvent) => {
      const delta = ev.clientX - startX;
      if (delta !== 0) {
        onResize(delta);
        startX = ev.clientX;
      }
    };

    const onMouseUp = () => {
      dragging = false;
      document.removeEventListener("mousemove", onMouseMove);
      document.removeEventListener("mouseup", onMouseUp);
    };

    document.addEventListener("mousemove", onMouseMove);
    document.addEventListener("mouseup", onMouseUp);
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="resizer"
  class:active={dragging}
  onmousedown={onMouseDown}
></div>

{#if dragging}
  <div class="drag-overlay"></div>
{/if}

<style>
  .resizer {
    width: 4px;
    cursor: col-resize;
    background: #333;
    flex-shrink: 0;
    transition: background 0.15s;
  }

  .resizer:hover,
  .resizer.active {
    background: #4A90D9;
  }

  .drag-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    z-index: 9999;
    cursor: col-resize;
    user-select: none;
  }
</style>
