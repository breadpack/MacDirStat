use std::collections::VecDeque;

use super::arena::{Node, NodeId, ScanTree, FLAG_DIR, FLAG_OTHERS, FLAG_REMOVED, NO_NODE};

/// 개별 노드로 유지할 파일 하나
pub struct FileEntry {
    pub name: String,
    pub size: u64,
    pub modified: u64,
    pub ext: Option<String>,
    pub flags: u8,
}

/// 디렉토리 하나의 파일 스캔 결과. 상위 N개 파일만 개별 엔트리로, 나머지는 합산값으로 전달된다.
pub struct DirResult {
    pub dir: NodeId,
    pub files: Vec<FileEntry>,
    pub others_count: u64,
    pub others_bytes: u64,
    pub others_modified: u64,
    pub bytes: u64,
    pub count: u64,
    pub modified: u64,
}

impl ScanTree {
    /// Phase 2 결과를 반영: 파일 노드를 연속 구간으로 추가하고 조상들의 크기를 갱신한다.
    pub fn apply_dir_result(&mut self, r: DirResult) {
        let start = self.nodes.len() as NodeId;
        for f in &r.files {
            let id = self.push_node(&f.name, r.dir, f.flags);
            let ext = f.ext.as_deref().map(|e| self.intern_ext(e)).unwrap_or(0);
            let n = self.node_mut(id);
            n.size = f.size;
            n.file_count = 1;
            n.modified = f.modified;
            n.ext = ext;
        }
        if r.others_count > 0 {
            let name = format!("({} other files)", r.others_count);
            let id = self.push_node(&name, r.dir, FLAG_OTHERS);
            let n = self.node_mut(id);
            n.size = r.others_bytes;
            n.file_count = r.others_count;
            n.modified = r.others_modified;
        }
        let len = self.nodes.len() as u32 - start;
        let dir = self.node_mut(r.dir);
        dir.files_start = start;
        dir.files_len = len;
        self.add_to_ancestors(r.dir, r.bytes, r.count, r.modified);
    }

    /// 노드를 트리에서 제거하고 조상들의 집계값을 차감한다.
    pub fn remove(&mut self, id: NodeId) -> bool {
        if id == 0 || self.node(id).has(FLAG_REMOVED) {
            return false;
        }
        let removed = *self.node(id);
        let dirs = if removed.is_dir() { 1 + removed.dir_count } else { 0 };
        self.garbage += self.subtree_len(id);
        self.node_mut(id).flags |= FLAG_REMOVED;

        let mut cur = removed.parent;
        while cur != NO_NODE {
            let n = self.node_mut(cur);
            n.size = n.size.saturating_sub(removed.size);
            n.file_count = n.file_count.saturating_sub(removed.file_count);
            n.dir_count = n.dir_count.saturating_sub(dirs);
            cur = n.parent;
        }
        self.maybe_compact();
        true
    }

    /// target 디렉토리의 서브트리를 새로 스캔한 sub 트리로 교체한다.
    pub fn graft(&mut self, target: NodeId, sub: &ScanTree) {
        let old = *self.node(target);
        self.garbage += self.subtree_len(target) - 1;

        // sub 노드 i(>=1) → base + i - 1, sub 루트(0) → target
        let base = self.nodes.len() as u32;
        let map = |i: u32| if i == 0 { target } else { base + i - 1 };
        let ext_map: Vec<u32> = sub.exts.iter().map(|e| self.intern_ext(e)).collect();

        self.nodes.reserve(sub.nodes.len().saturating_sub(1));
        for i in 1..sub.nodes.len() as u32 {
            let sn = *sub.node(i);
            let (off, len) = self.push_name(sub.name(i));
            self.nodes.push(Node {
                parent: map(sn.parent),
                name_off: off,
                name_len: len,
                dirs_start: if sn.dirs_len > 0 { map(sn.dirs_start) } else { 0 },
                files_start: if sn.files_len > 0 { map(sn.files_start) } else { 0 },
                ext: if sn.ext > 0 { ext_map[sn.ext as usize - 1] } else { 0 },
                ..sn
            });
        }

        let sr = *sub.node(0);
        let t = self.node_mut(target);
        t.dirs_start = if sr.dirs_len > 0 { map(sr.dirs_start) } else { 0 };
        t.dirs_len = sr.dirs_len;
        t.files_start = if sr.files_len > 0 { map(sr.files_start) } else { 0 };
        t.files_len = sr.files_len;
        t.size = sr.size;
        t.file_count = sr.file_count;
        t.dir_count = sr.dir_count;
        t.modified = sr.modified;

        let mut cur = old.parent;
        while cur != NO_NODE {
            let n = self.node_mut(cur);
            n.size = (n.size + sr.size).saturating_sub(old.size);
            n.file_count = (n.file_count + sr.file_count).saturating_sub(old.file_count);
            n.dir_count = (n.dir_count + sr.dir_count).saturating_sub(old.dir_count);
            n.modified = n.modified.max(sr.modified);
            cur = n.parent;
        }
        self.maybe_compact();
    }

    fn maybe_compact(&mut self) {
        if self.garbage > 0 && self.garbage * 3 > self.nodes.len() {
            self.compact();
        }
    }

    /// 도달 가능한 노드만으로 arena를 재구성한다 (BFS, 자식 구간 연속성 유지).
    pub fn compact(&mut self) {
        let mut nodes: Vec<Node> = Vec::with_capacity(self.nodes.len() - self.garbage.min(self.nodes.len()));
        let mut names: Vec<u8> = Vec::with_capacity(self.names.len());
        let copy = |names: &mut Vec<u8>, src: &ScanTree, old: NodeId, parent: NodeId| -> Node {
            let off = names.len() as u32;
            names.extend_from_slice(src.name(old).as_bytes());
            Node { parent, name_off: off, dirs_start: 0, dirs_len: 0, files_start: 0, files_len: 0, ..*src.node(old) }
        };

        nodes.push(copy(&mut names, self, 0, NO_NODE));
        let mut queue: VecDeque<(NodeId, NodeId)> = VecDeque::from([(0, 0)]);
        while let Some((old, new)) = queue.pop_front() {
            let on = *self.node(old);
            let live = |range: std::ops::Range<u32>| range.filter(|&c| !self.node(c).has(FLAG_REMOVED));

            let dirs_start = nodes.len() as NodeId;
            for c in live(on.dirs_start..on.dirs_start + on.dirs_len) {
                let id = nodes.len() as NodeId;
                nodes.push(copy(&mut names, self, c, new));
                queue.push_back((c, id));
            }
            let dirs_len = nodes.len() as u32 - dirs_start;

            let files_start = nodes.len() as NodeId;
            for c in live(on.files_start..on.files_start + on.files_len) {
                nodes.push(copy(&mut names, self, c, new));
            }
            let files_len = nodes.len() as u32 - files_start;

            let n = &mut nodes[new as usize];
            n.dirs_start = if dirs_len > 0 { dirs_start } else { 0 };
            n.dirs_len = dirs_len;
            n.files_start = if files_len > 0 { files_start } else { 0 };
            n.files_len = files_len;
        }

        debug_assert!(nodes.iter().skip(1).all(|n| n.flags & FLAG_REMOVED == 0));
        debug_assert!(nodes[0].flags & FLAG_DIR != 0);
        nodes.shrink_to_fit();
        names.shrink_to_fit();
        self.nodes = nodes;
        self.names = names;
        self.garbage = 0;
    }
}
