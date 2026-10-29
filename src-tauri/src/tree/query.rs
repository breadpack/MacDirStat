use std::collections::BinaryHeap;

use super::arena::{push_segment, NodeId, ScanTree, FLAG_HIDDEN, FLAG_OTHERS, FLAG_READONLY, FLAG_SYMLINK, OTHERS_SEGMENT};
use crate::models::{ExtensionStat, NodeDto, TreemapNode};

/// 합산 노드(other files)는 확장자 통계에서 이 키로 집계된다 (프론트엔드 규칙과 동일)
const NO_EXT: &str = "(no ext)";

impl ScanTree {
    pub fn dto(&self, id: NodeId, path: String) -> NodeDto {
        let is_dir = self.is_dir(id);
        let modified = self.modified(id);
        NodeDto {
            name: self.name(id).into_owned(),
            path,
            size: self.size(id),
            is_dir,
            file_count: self.file_count(id),
            extension: self.ext(id).map(str::to_string),
            dir_count: if is_dir { self.dir(id).dir_count as u64 } else { 0 },
            modified: if modified > 0 { Some(modified) } else { None },
            is_symlink: self.has(id, FLAG_SYMLINK),
            is_hidden: self.has(id, FLAG_HIDDEN),
            is_readonly: self.has(id, FLAG_READONLY),
            cleanup_pattern_id: self.cleanup_id(id).map(str::to_string),
            child_count: if is_dir { self.child_count(id) } else { 0 },
        }
    }

    pub fn node_dto(&self, path: &str) -> Option<NodeDto> {
        let (id, parent) = self.resolve_entry(path)?;
        Some(self.dto(id, self.entry_path(id, parent)))
    }

    pub fn root_dto(&self) -> NodeDto {
        self.dto(0, self.root_path.clone())
    }

    pub fn children_dtos(&self, path: &str) -> Option<Vec<NodeDto>> {
        let id = self.resolve(path)?;
        if !self.is_dir(id) {
            return Some(Vec::new());
        }
        let parent_path = self.path_of(id);
        Some(
            self.children(id)
                .map(|c| self.dto(c, self.child_path(&parent_path, c)))
                .collect(),
        )
    }

    /// 트리맵 렌더링용 경량 트리. 큰 디렉토리부터 우선 확장하여 노드 수를 max_nodes로 제한한다.
    pub fn treemap(&self, path: &str, min_ratio: f64, max_children: usize, max_nodes: usize) -> Option<TreemapNode> {
        struct Flat {
            node: TreemapNode,
            kids: Vec<usize>,
        }

        let (root, root_parent) = self.resolve_entry(path)?;
        let root_size = self.size(root);
        let min_size = ((root_size as f64 * min_ratio) as u64).max(1);

        let mut out: Vec<Flat> = vec![Flat { node: self.treemap_leaf(root, self.entry_path(root, root_parent)), kids: Vec::new() }];
        // (size, out index, node id) — 가장 큰 디렉토리부터 확장
        let mut heap: BinaryHeap<(u64, usize, NodeId)> = BinaryHeap::new();
        if self.is_dir(root) {
            heap.push((root_size, 0, root));
        }

        while let Some((_, oi, id)) = heap.pop() {
            if out.len() >= max_nodes {
                break;
            }
            let mut kept: Vec<NodeId> = self.children(id).filter(|&c| self.size(c) >= min_size).collect();
            if kept.is_empty() {
                continue;
            }
            kept.sort_unstable_by(|&a, &b| self.size(b).cmp(&self.size(a)));
            let total_children = self.child_count(id) as usize;
            kept.truncate(max_children);

            let parent_path = out[oi].node.path.clone();
            let mut kept_size = 0u64;
            for &c in &kept {
                let c_size = self.size(c);
                kept_size += c_size;
                let idx = out.len();
                out.push(Flat { node: self.treemap_leaf(c, self.child_path(&parent_path, c)), kids: Vec::new() });
                out[oi].kids.push(idx);
                if self.is_dir(c) && c_size > 0 && self.has_any_child(c) {
                    heap.push((c_size, idx, c));
                }
            }

            let rest = self.size(id).saturating_sub(kept_size);
            if rest > 0 {
                let mut others_path = parent_path;
                push_segment(&mut others_path, OTHERS_SEGMENT);
                let idx = out.len();
                out.push(Flat {
                    node: TreemapNode {
                        name: format!("({} others)", total_children - kept.len()),
                        path: others_path,
                        size: rest,
                        is_dir: false,
                        extension: None,
                        cleanup_pattern_id: None,
                        children: Vec::new(),
                    },
                    kids: Vec::new(),
                });
                out[oi].kids.push(idx);
            }
        }

        // 자식 인덱스는 항상 부모보다 크므로 역순으로 조립
        let mut built: Vec<Option<TreemapNode>> = Vec::with_capacity(out.len());
        let mut kids_list: Vec<Vec<usize>> = Vec::with_capacity(out.len());
        for f in out {
            built.push(Some(f.node));
            kids_list.push(f.kids);
        }
        for i in (0..built.len()).rev() {
            if kids_list[i].is_empty() {
                continue;
            }
            let children: Vec<TreemapNode> = kids_list[i].iter().filter_map(|&k| built[k].take()).collect();
            if let Some(n) = built[i].as_mut() {
                n.children = children;
            }
        }
        built[0].take()
    }

    fn treemap_leaf(&self, id: NodeId, path: String) -> TreemapNode {
        TreemapNode {
            name: self.name(id).into_owned(),
            path,
            size: self.size(id),
            is_dir: self.is_dir(id),
            extension: self.ext(id).map(str::to_string),
            cleanup_pattern_id: self.cleanup_id(id).map(str::to_string),
            children: Vec::new(),
        }
    }

    fn has_any_child(&self, dir: NodeId) -> bool {
        let d = self.dir(dir);
        d.dirs_len > 0 || d.files_len > 0
    }

    /// path 하위 파일들의 확장자별 크기/개수 (bytes 내림차순)
    pub fn extension_stats(&self, path: &str) -> Vec<ExtensionStat> {
        let Some(root) = self.resolve(path) else { return Vec::new() };
        // index 0 = 확장자 없음, i = exts[i - 1]
        let mut acc: Vec<(u64, u64)> = vec![(0, 0); self.exts.len() + 1];
        let mut stack = vec![root];
        while let Some(id) = stack.pop() {
            if self.is_dir(id) {
                stack.extend(self.children(id));
            } else {
                let f = self.file(id);
                let slot = if f.flags & FLAG_OTHERS != 0 { 0 } else { f.ext as usize };
                acc[slot].0 += f.size;
                acc[slot].1 += self.file_count(id);
            }
        }
        let mut stats: Vec<ExtensionStat> = acc
            .into_iter()
            .enumerate()
            .filter(|(_, (_, count))| *count > 0)
            .map(|(i, (bytes, count))| ExtensionStat {
                extension: if i == 0 { NO_EXT.to_string() } else { self.exts[i - 1].to_lowercase() },
                total_bytes: bytes,
                file_count: count,
            })
            .collect();
        stats.sort_unstable_by(|a, b| b.total_bytes.cmp(&a.total_bytes));
        stats
    }
}
