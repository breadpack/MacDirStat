use std::collections::VecDeque;

use super::arena::{clamp_secs, file_id, is_file_id, DirNode, FileNode, NodeId, ScanTree, FLAG_OTHERS, FLAG_REMOVED, NO_NODE};
use super::chunked::{ChunkedVec, NamePool};

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
        let start = self.files.len() as u32;
        for f in &r.files {
            let id = self.push_file(&f.name, f.ext.as_deref(), f.flags);
            let n = self.file_mut(id);
            n.size = f.size;
            n.modified = clamp_secs(f.modified);
        }
        if r.others_count > 0 {
            let name = format!("({} other files)", r.others_count);
            let id = self.push_file(&name, None, FLAG_OTHERS);
            let n = self.file_mut(id);
            n.size = r.others_bytes;
            n.modified = clamp_secs(r.others_modified);
            n.ext = r.others_count.min(u32::MAX as u64) as u32;
        }
        let len = self.files.len() as u32 - start;
        let dir = self.dir_mut(r.dir);
        dir.files_start = start;
        dir.files_len = len;
        self.add_to_ancestors(r.dir, r.bytes, r.count, r.modified);
    }

    /// 경로의 노드를 트리에서 제거하고 조상들의 집계값을 차감한다.
    pub fn remove_path(&mut self, path: &str) -> bool {
        match self.resolve_entry(path) {
            Some((id, parent)) => self.remove_entry(id, parent),
            None => false,
        }
    }

    fn remove_entry(&mut self, id: NodeId, parent: NodeId) -> bool {
        if id == 0 || parent == NO_NODE || self.has(id, FLAG_REMOVED) {
            return false;
        }
        let size = self.size(id);
        let files = self.file_count(id);
        let dirs = if is_file_id(id) { 0 } else { 1 + self.dir(id).dir_count };
        self.garbage += self.subtree_len(id);
        self.set_flag(id, FLAG_REMOVED);

        let mut cur = parent;
        while cur != NO_NODE {
            let n = self.dir_mut(cur);
            n.size = n.size.saturating_sub(size);
            n.file_count = n.file_count.saturating_sub(files);
            n.dir_count = n.dir_count.saturating_sub(dirs);
            cur = n.parent;
        }
        self.maybe_compact();
        true
    }

    /// target 디렉토리의 서브트리를 새로 스캔한 sub 트리로 교체한다.
    pub fn graft(&mut self, target: NodeId, sub: &ScanTree) {
        let old = *self.dir(target);
        self.garbage += self.subtree_len(target) - 1;

        // sub 디렉토리 i(>=1) → dir_base + i - 1, sub 루트(0) → target. 파일 j → file_base + j
        let dir_base = self.dirs.len() as u32;
        let file_base = self.files.len() as u32;
        let map_dir = |i: u32| if i == 0 { target } else { dir_base + i - 1 };
        let ext_map: Vec<u32> = sub.exts.iter().map(|e| self.intern_ext(e)).collect();

        for i in 1..sub.dirs.len() as u32 {
            let sd = *sub.dir(i);
            let (name_off, name_len) = self.push_name(sub.raw_name(i));
            self.dirs.push(DirNode {
                parent: map_dir(sd.parent),
                name_off,
                name_len,
                dirs_start: if sd.dirs_len > 0 { map_dir(sd.dirs_start) } else { 0 },
                files_start: if sd.files_len > 0 { file_base + sd.files_start } else { 0 },
                ..sd
            });
        }
        for j in 0..sub.files.len() as u32 {
            let sf = sub.files[j as usize];
            // 확장자가 떼어진 stem 그대로 복사 (FLAG_EXT_STRIPPED 유지)
            let (name_off, name_len) = self.push_name(sub.raw_name(file_id(j)));
            let ext = if sf.flags & FLAG_OTHERS == 0 && sf.ext > 0 { ext_map[sf.ext as usize - 1] } else { sf.ext };
            self.files.push(FileNode { name_off, name_len, ext, ..sf });
        }

        let sr = *sub.dir(0);
        let t = self.dir_mut(target);
        t.dirs_start = if sr.dirs_len > 0 { map_dir(sr.dirs_start) } else { 0 };
        t.dirs_len = sr.dirs_len;
        t.files_start = if sr.files_len > 0 { file_base + sr.files_start } else { 0 };
        t.files_len = sr.files_len;
        t.size = sr.size;
        t.file_count = sr.file_count;
        t.dir_count = sr.dir_count;
        t.modified = sr.modified;

        let mut cur = old.parent;
        while cur != NO_NODE {
            let n = self.dir_mut(cur);
            n.size = (n.size + sr.size).saturating_sub(old.size);
            n.file_count = (n.file_count + sr.file_count).saturating_sub(old.file_count);
            n.dir_count = (n.dir_count + sr.dir_count).saturating_sub(old.dir_count);
            n.modified = n.modified.max(sr.modified);
            cur = n.parent;
        }
        self.maybe_compact();
    }

    fn maybe_compact(&mut self) {
        if self.garbage > 0 && self.garbage * 3 > self.len() {
            self.compact();
        }
    }

    /// 도달 가능한 노드만으로 arena를 재구성한다 (BFS, 자식 구간 연속성 유지).
    pub fn compact(&mut self) {
        let mut dirs: ChunkedVec<DirNode> = ChunkedVec::new();
        let mut files: ChunkedVec<FileNode> = ChunkedVec::new();
        let mut names = NamePool::new();
        let copy_name = |names: &mut NamePool, src: &ScanTree, id: NodeId| -> u32 { names.push(src.raw_name(id).as_bytes()) };

        let root = DirNode { name_off: copy_name(&mut names, self, 0), ..*self.dir(0) };
        dirs.push(root);
        let mut queue: VecDeque<(NodeId, NodeId)> = VecDeque::from([(0, 0)]);
        while let Some((old, new)) = queue.pop_front() {
            let od = *self.dir(old);

            let dirs_start = dirs.len() as u32;
            for c in od.dirs_start..od.dirs_start + od.dirs_len {
                if self.dirs[c as usize].flags & FLAG_REMOVED != 0 {
                    continue;
                }
                let id = dirs.push(DirNode { parent: new, name_off: copy_name(&mut names, self, c), ..self.dirs[c as usize] });
                queue.push_back((c, id as NodeId));
            }
            let dirs_len = dirs.len() as u32 - dirs_start;

            let files_start = files.len() as u32;
            for c in od.files_start..od.files_start + od.files_len {
                if self.files[c as usize].flags & FLAG_REMOVED != 0 {
                    continue;
                }
                files.push(FileNode { name_off: copy_name(&mut names, self, file_id(c)), ..self.files[c as usize] });
            }
            let files_len = files.len() as u32 - files_start;

            let n = &mut dirs[new as usize];
            n.dirs_start = if dirs_len > 0 { dirs_start } else { 0 };
            n.dirs_len = dirs_len;
            n.files_start = if files_len > 0 { files_start } else { 0 };
            n.files_len = files_len;
        }

        dirs.shrink_to_fit();
        files.shrink_to_fit();
        names.shrink_to_fit();
        self.dirs = dirs;
        self.files = files;
        self.names = names;
        self.garbage = 0;
    }
}
