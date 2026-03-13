import type { FileNode } from "../types";

/**
 * Find a node by path in the tree using iterative DFS.
 * Returns null if not found.
 */
export function findNode(root: FileNode, targetPath: string): FileNode | null {
  const stack: FileNode[] = [root];
  while (stack.length > 0) {
    const node = stack.pop()!;
    if (node.path === targetPath) return node;
    if (targetPath.startsWith(node.path + "/") || node.path === "/") {
      for (const child of node.children) {
        stack.push(child);
      }
    }
  }
  return null;
}

/**
 * Replace a subtree at `targetPath` with `newSubtree`, recalculating
 * size/file_count/dir_count for all ancestors up to the root.
 * Uses iterative path-segment walking + upward rebuild.
 */
export function replaceSubtree(
  node: FileNode,
  targetPath: string,
  newSubtree: FileNode,
): FileNode {
  if (node.path === targetPath) return newSubtree;

  // Walk down the tree following path segments to collect ancestor chain
  const ancestors: FileNode[] = [node];
  const childIndices: number[] = [];
  let current = node;

  while (current.path !== targetPath) {
    let found = false;
    for (let i = 0; i < current.children.length; i++) {
      const child = current.children[i];
      if (
        child.path === targetPath ||
        targetPath.startsWith(child.path + "/") ||
        child.path === "/"
      ) {
        childIndices.push(i);
        ancestors.push(child);
        current = child;
        found = true;
        break;
      }
    }
    if (!found) return node; // target not in this subtree
  }

  // Rebuild upward from the target
  let rebuilt: FileNode = newSubtree;
  for (let i = ancestors.length - 2; i >= 0; i--) {
    const ancestor = ancestors[i];
    const idx = childIndices[i];
    const newChildren = [...ancestor.children];
    newChildren[idx] = rebuilt;

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
      }, null) ?? ancestor.modified;

    rebuilt = { ...ancestor, children: newChildren, size, file_count, dir_count, modified };
  }

  return rebuilt;
}

/**
 * Immutably remove a node from the tree, returning a new tree with updated
 * sizes/counts. Returns null if the target was not found.
 * Uses iterative path-walking + upward rebuild.
 */
export function removeNode(node: FileNode, targetPath: string): FileNode | null {
  // Walk down following path segments to find the parent of the target
  const ancestors: FileNode[] = [node];
  const childIndices: number[] = [];
  let current = node;

  // eslint-disable-next-line no-constant-condition
  while (true) {
    const idx = current.children.findIndex((c) => c.path === targetPath);
    if (idx >= 0) {
      // Found the target as a direct child of current
      const removed = current.children[idx];
      const newChildren = current.children.filter((_, i) => i !== idx);

      // Rebuild current node with updated stats
      let rebuilt: FileNode = {
        ...current,
        children: newChildren,
        size: current.size - removed.size,
        file_count: current.file_count - (removed.is_dir ? removed.file_count : 1),
        dir_count: current.dir_count - (removed.is_dir ? 1 + removed.dir_count : 0),
      };

      // Rebuild upward through ancestors
      for (let i = ancestors.length - 2; i >= 0; i--) {
        const ancestor = ancestors[i];
        const ancIdx = childIndices[i];
        const sizeDiff = ancestor.children[ancIdx].size - rebuilt.size;
        const fileDiff = ancestor.children[ancIdx].file_count - rebuilt.file_count;
        const dirDiff = ancestor.children[ancIdx].dir_count - rebuilt.dir_count;
        const ancChildren = [...ancestor.children];
        ancChildren[ancIdx] = rebuilt;
        rebuilt = {
          ...ancestor,
          children: ancChildren,
          size: ancestor.size - sizeDiff,
          file_count: ancestor.file_count - fileDiff,
          dir_count: ancestor.dir_count - dirDiff,
        };
      }

      return rebuilt;
    }

    // Not a direct child; find which child to descend into
    let found = false;
    for (let i = 0; i < current.children.length; i++) {
      const child = current.children[i];
      if (child.children && (targetPath.startsWith(child.path + "/") || child.path === "/")) {
        childIndices.push(i);
        ancestors.push(child);
        current = child;
        found = true;
        break;
      }
    }
    if (!found) return null; // target not found
  }
}
