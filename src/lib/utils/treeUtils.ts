import type { FileNode } from "../types";

/**
 * Find a node by path in the tree. Returns null if not found.
 */
export function findNode(node: FileNode, targetPath: string): FileNode | null {
  if (node.path === targetPath) return node;
  if (!targetPath.startsWith(node.path + "/") && node.path !== "/") return null;
  for (const child of node.children) {
    const found = findNode(child, targetPath);
    if (found) return found;
  }
  return null;
}

/**
 * Replace a subtree at `targetPath` with `newSubtree`, recalculating
 * size/file_count/dir_count for all ancestors up to the root.
 */
export function replaceSubtree(
  node: FileNode,
  targetPath: string,
  newSubtree: FileNode,
): FileNode {
  if (node.path === targetPath) return newSubtree;
  if (!targetPath.startsWith(node.path + "/") && node.path !== "/") return node;

  const newChildren = node.children.map((child) =>
    replaceSubtree(child, targetPath, newSubtree),
  );

  const size = newChildren.reduce((s, c) => s + c.size, 0);
  const file_count = newChildren.reduce((s, c) => s + c.file_count, 0);
  const dir_count = newChildren
    .filter((c) => c.is_dir)
    .reduce((s, c) => s + 1 + c.dir_count, 0);
  const modified =
    newChildren.reduce<number | null>((max, c) => {
      if (c.modified == null) return max;
      if (max == null) return c.modified;
      return c.modified > max ? c.modified : max;
    }, null) ?? node.modified;

  return { ...node, children: newChildren, size, file_count, dir_count, modified };
}
