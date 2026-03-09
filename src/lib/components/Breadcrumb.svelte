<script lang="ts">
  import { tree } from "../stores/scanStore";
  import { zoomRoot } from "../stores/selectionStore";
  import type { FileNode } from "../types";

  interface Crumb {
    name: string;
    path: string;
  }

  let crumbs: Crumb[] = $derived.by(() => {
    const t = $tree;
    const zr = $zoomRoot;
    if (!t) return [];

    const result: Crumb[] = [{ name: t.name, path: t.path }];
    if (!zr || zr === t.path) return result;

    // Build path from root to zoom target
    const parts = findPath(t, zr);
    if (parts) {
      result.push(...parts);
    }
    return result;
  });

  function findPath(node: FileNode, target: string): Crumb[] | null {
    for (const child of node.children) {
      if (child.path === target) {
        return [{ name: child.name, path: child.path }];
      }
      if (child.is_dir) {
        const sub = findPath(child, target);
        if (sub) {
          return [{ name: child.name, path: child.path }, ...sub];
        }
      }
    }
    return null;
  }

  function navigate(path: string) {
    const t = $tree;
    if (t && path === t.path) {
      zoomRoot.set(null);
    } else {
      zoomRoot.set(path);
    }
  }
</script>

{#if crumbs.length > 0}
  <div class="breadcrumb">
    {#each crumbs as crumb, i}
      {#if i > 0}
        <span class="separator">/</span>
      {/if}
      <button class="crumb" onclick={() => navigate(crumb.path)}>
        {crumb.name}
      </button>
    {/each}
  </div>
{/if}

<style>
  .breadcrumb {
    display: flex;
    align-items: center;
    padding: 4px 8px;
    background: #1a1a1a;
    border-bottom: 1px solid #333;
    font-size: 12px;
    overflow-x: auto;
    white-space: nowrap;
  }

  .separator {
    color: #666;
    margin: 0 4px;
  }

  .crumb {
    background: none;
    border: none;
    color: #4A90D9;
    cursor: pointer;
    padding: 2px 4px;
    border-radius: 2px;
    font-size: 12px;
  }

  .crumb:hover {
    background: #2a2a2a;
    color: #6BA5DA;
  }

  .crumb:last-child {
    color: #ccc;
  }
</style>
