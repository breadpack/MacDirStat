import { hierarchy, treemap, treemapSquarify } from "d3-hierarchy";
import type { TreemapNode } from "../types";

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
  data: TreemapNode;
  depth: number;
  ancestors: AncestorRect[];
}

const MAX_LEAVES = 5000;

export function computeTreemap(
  root: TreemapNode,
  width: number,
  height: number,
  padding: number = 1,
): TreemapRect[] {
  // root는 백엔드에서 이미 가지치기된 트리. 자식이 없는 노드(펼치지 않은 디렉토리 포함)는 leaf로 취급한다.
  const h = hierarchy(root, (d) => (d.children.length > 0 ? d.children : undefined))
    .sum((d) => (d.children.length > 0 ? 0 : d.size))
    .sort((a, b) => (b.value ?? 0) - (a.value ?? 0));

  const layout = treemap<TreemapNode>()
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
