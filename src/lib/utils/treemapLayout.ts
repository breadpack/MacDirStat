import { hierarchy, treemap, treemapSquarify } from "d3-hierarchy";
import type { FileNode } from "../types";

export interface AncestorRect {
  x0: number;
  y0: number;
  x1: number;
  y1: number;
  depth: number;
}

export interface TreemapRect {
  x0: number;
  y0: number;
  x1: number;
  y1: number;
  data: FileNode;
  depth: number;
  ancestors: AncestorRect[];
}

const MAX_LEAVES = 5000;

/** Prune tree to limit total nodes: keep top children by size at each level */
function pruneTree(node: FileNode, minSize: number): FileNode {
  if (!node.is_dir || node.children.length === 0) return node;

  const kept = node.children
    .filter((c) => c.size >= minSize)
    .slice(0, 200)
    .map((c) => (c.is_dir ? pruneTree(c, minSize) : c));

  const keptSize = kept.reduce((s, c) => s + c.size, 0);
  const otherSize = node.size - keptSize;

  if (otherSize > 0 && kept.length > 0) {
    kept.push({
      name: `(${node.children.length - kept.length} others)`,
      path: node.path + "/__others__",
      size: otherSize,
      is_dir: false,
      children: [],
      file_count: 0,
      extension: null,
      dir_count: 0,
      modified: null,
      is_symlink: false,
      is_hidden: false,
      is_readonly: false,
    });
  }

  return { ...node, children: kept };
}

export function computeTreemap(
  root: FileNode,
  width: number,
  height: number,
  padding: number = 1,
): TreemapRect[] {
  // Prune small files to prevent processing millions of nodes
  const minSize = Math.max(1, Math.floor(root.size * 0.0001));
  const pruned = pruneTree(root, minSize);

  const h = hierarchy(pruned, (d) => (d.is_dir ? d.children : undefined))
    .sum((d) => (d.is_dir ? 0 : d.size))
    .sort((a, b) => (b.value ?? 0) - (a.value ?? 0));

  const layout = treemap<FileNode>()
    .tile(treemapSquarify)
    .size([width, height])
    .padding(padding)
    .round(true);

  const treeRoot = layout(h);

  const rects: TreemapRect[] = [];
  for (const leaf of treeRoot.leaves()) {
    if (rects.length >= MAX_LEAVES) break;

    // Collect ancestor boundaries (root -> parent order, excluding the leaf itself)
    const ancestors: AncestorRect[] = [];
    for (const anc of leaf.ancestors()) {
      if (anc === leaf) continue;
      ancestors.push({
        x0: anc.x0,
        y0: anc.y0,
        x1: anc.x1,
        y1: anc.y1,
        depth: anc.depth,
      });
    }
    // Reverse so order is root -> parent (ancestors() returns leaf -> root)
    ancestors.reverse();

    rects.push({
      x0: leaf.x0,
      y0: leaf.y0,
      x1: leaf.x1,
      y1: leaf.y1,
      data: leaf.data,
      depth: leaf.depth,
      ancestors,
    });
  }

  return rects;
}
