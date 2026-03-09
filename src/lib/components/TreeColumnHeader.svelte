<script lang="ts">
  import { columns, sortState, type ColumnDef } from "../stores/columnStore";

  let columnDefs = $state<ColumnDef[]>([]);
  let currentSort = $state<{ columnId: string; direction: "asc" | "desc" }>({
    columnId: "size",
    direction: "desc",
  });

  $effect(() => {
    const unsub = columns.subscribe((v) => {
      columnDefs = v;
    });
    return unsub;
  });

  $effect(() => {
    const unsub = sortState.subscribe((v) => {
      currentSort = v;
    });
    return unsub;
  });

  let visibleColumns = $derived(columnDefs.filter((c) => c.visible));

  // Drag resize state
  let dragging = $state<{ columnId: string; startX: number; startWidth: number } | null>(null);

  // Context menu state
  let ctxMenu = $state<{ x: number; y: number } | null>(null);

  function handleHeaderClick(col: ColumnDef) {
    if (col.sortable) {
      sortState.toggleSort(col.id);
    }
  }

  function handleResizeStart(e: MouseEvent, col: ColumnDef) {
    e.preventDefault();
    e.stopPropagation();
    dragging = { columnId: col.id, startX: e.clientX, startWidth: col.width };

    function onMouseMove(ev: MouseEvent) {
      if (!dragging) return;
      const diff = ev.clientX - dragging.startX;
      columns.setWidth(dragging.columnId, dragging.startWidth + diff);
    }

    function onMouseUp() {
      dragging = null;
      document.removeEventListener("mousemove", onMouseMove);
      document.removeEventListener("mouseup", onMouseUp);
    }

    document.addEventListener("mousemove", onMouseMove);
    document.addEventListener("mouseup", onMouseUp);
  }

  function handleRightClick(e: MouseEvent) {
    e.preventDefault();
    ctxMenu = { x: e.clientX, y: e.clientY };
  }

  function closeCtxMenu() {
    ctxMenu = null;
  }

  function toggleColumn(id: string) {
    columns.toggleVisibility(id);
    ctxMenu = null;
  }

  function getSortIndicator(col: ColumnDef): string {
    if (currentSort.columnId !== col.id) return "";
    return currentSort.direction === "asc" ? " \u25B2" : " \u25BC";
  }
</script>

<svelte:window
  onclick={() => {
    if (ctxMenu) closeCtxMenu();
  }}
/>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="column-header" oncontextmenu={handleRightClick}>
  {#each visibleColumns as col (col.id)}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <div
      class="header-cell"
      class:sortable={col.sortable}
      class:active={currentSort.columnId === col.id}
      style="width: {col.width}px; text-align: {col.align}; {col.id === 'name' ? 'flex: 1; min-width: ' + col.minWidth + 'px;' : 'flex-shrink: 0;'}"
      onclick={() => handleHeaderClick(col)}
    >
      <span class="header-label">{col.label}{getSortIndicator(col)}</span>
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      {#if col.id !== "name"}
        <div
          class="resize-handle"
          onmousedown={(e) => handleResizeStart(e, col)}
        ></div>
      {/if}
    </div>
  {/each}
</div>

{#if dragging}
  <div class="drag-overlay"></div>
{/if}

{#if ctxMenu}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div
    class="ctx-menu"
    style="left: {ctxMenu.x}px; top: {ctxMenu.y}px"
    onclick={(e) => e.stopPropagation()}
  >
    {#each columnDefs as col (col.id)}
      {#if col.id !== "name"}
        <label class="ctx-item">
          <input
            type="checkbox"
            checked={col.visible}
            onchange={() => toggleColumn(col.id)}
          />
          {col.label}
        </label>
      {/if}
    {/each}
  </div>
{/if}

<style>
  .column-header {
    display: flex;
    align-items: center;
    height: 28px;
    background: #252525;
    border-bottom: 1px solid #333;
    font-size: 12px;
    color: #aaa;
    user-select: none;
    flex-shrink: 0;
  }

  .header-cell {
    display: flex;
    align-items: center;
    height: 100%;
    padding: 0 6px;
    position: relative;
    overflow: hidden;
    white-space: nowrap;
  }

  .header-cell.sortable {
    cursor: pointer;
  }

  .header-cell.sortable:hover {
    color: #ddd;
    background: #2a2a2a;
  }

  .header-cell.active {
    color: #ddd;
  }

  .header-label {
    overflow: hidden;
    text-overflow: ellipsis;
    flex: 1;
  }

  .resize-handle {
    position: absolute;
    right: 0;
    top: 0;
    width: 5px;
    height: 100%;
    cursor: col-resize;
    z-index: 1;
  }

  .resize-handle:hover {
    background: rgba(74, 144, 217, 0.4);
  }

  .drag-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    cursor: col-resize;
    z-index: 9999;
  }

  .ctx-menu {
    position: fixed;
    background: #2a2a2a;
    border: 1px solid #444;
    border-radius: 4px;
    padding: 4px 0;
    z-index: 10000;
    min-width: 140px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.5);
  }

  .ctx-item {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 12px;
    font-size: 12px;
    color: #ccc;
    cursor: pointer;
  }

  .ctx-item:hover {
    background: #333;
  }

  .ctx-item input[type="checkbox"] {
    accent-color: #4a90d9;
  }
</style>
