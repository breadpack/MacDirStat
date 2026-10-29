use std::borrow::Cow;
use std::collections::HashMap;

use super::chunked::{ChunkedVec, NamePool};

/// 노드 식별자. FILE_BIT가 켜져 있으면 하위 31비트가 files 인덱스, 아니면 dirs 인덱스.
pub type NodeId = u32;
pub const NO_NODE: NodeId = u32::MAX;
pub const FILE_BIT: u32 = 1 << 31;

pub const FLAG_DIR: u8 = 1;
pub const FLAG_SYMLINK: u8 = 2;
pub const FLAG_HIDDEN: u8 = 4;
pub const FLAG_READONLY: u8 = 8;
/// 디렉토리당 상한을 넘어 개별 노드로 저장하지 않은 파일들의 합산 노드
pub const FLAG_OTHERS: u8 = 16;
pub const FLAG_REMOVED: u8 = 32;
/// 이름 풀에 확장자를 뗀 stem만 저장됨 (표시 이름 = stem + "." + ext)
pub const FLAG_EXT_STRIPPED: u8 = 64;

/// 경로 세그먼트로 사용되는 합산 노드의 가상 이름
pub const OTHERS_SEGMENT: &str = "__others__";

/// 디렉토리 노드 (64바이트). 경로는 저장하지 않고 parent 체인으로 계산한다.
/// 자식은 arena 내 연속 구간(dirs / files)으로 표현하여 노드별 Vec 할당을 없앤다.
#[derive(Clone, Copy, Debug)]
pub struct DirNode {
    pub size: u64,
    pub file_count: u64,
    /// unix seconds, 0 = unknown
    pub modified: u64,
    pub parent: NodeId,
    pub name_off: u32,
    /// dirs 인덱스
    pub dirs_start: u32,
    pub dirs_len: u32,
    /// files 인덱스 (FILE_BIT 없음)
    pub files_start: u32,
    pub files_len: u32,
    /// 하위 디렉토리 총 개수
    pub dir_count: u32,
    pub name_len: u16,
    /// 0 = 없음, 그 외 cleanup pattern 인덱스 + 1
    pub cleanup: u16,
    pub flags: u8,
}

/// 파일 노드 (24바이트). 노드 대부분이 파일이므로 디렉토리 전용 필드와 parent를 갖지 않는다.
/// 부모가 필요한 연산(경로 계산, 삭제)은 경로를 따라 내려오며 부모를 함께 얻는다.
#[derive(Clone, Copy, Debug)]
pub struct FileNode {
    pub size: u64,
    pub name_off: u32,
    /// unix seconds, 0 = unknown
    pub modified: u32,
    /// 일반 파일: 0 = 확장자 없음, 그 외 exts 인덱스 + 1.
    /// FLAG_OTHERS 노드: 합산된 파일 수.
    pub ext: u32,
    pub name_len: u16,
    pub flags: u8,
}

/// 스캔 결과 전체를 담는 arena. dirs[0]이 루트.
pub struct ScanTree {
    pub(crate) dirs: ChunkedVec<DirNode>,
    pub(crate) files: ChunkedVec<FileNode>,
    pub(crate) names: NamePool,
    pub(crate) exts: Vec<Box<str>>,
    ext_index: HashMap<Box<str>, u32>,
    pub(crate) cleanup_ids: Vec<&'static str>,
    pub(crate) root_path: String,
    /// 더 이상 도달할 수 없는 노드 수 (삭제/서브트리 교체로 발생)
    pub(crate) garbage: usize,
}

#[inline]
pub fn is_file_id(id: NodeId) -> bool {
    id != NO_NODE && id & FILE_BIT != 0
}

#[inline]
pub fn file_id(index: u32) -> NodeId {
    index | FILE_BIT
}

pub(crate) fn clamp_secs(secs: u64) -> u32 {
    secs.min(u32::MAX as u64) as u32
}

/// name이 ".{ext}"로 끝나면 stem을 반환 (대소문자까지 일치할 때만 — 복원 시 원래 이름과 같아야 하므로)
pub(crate) fn strip_ext<'a>(name: &'a str, ext: &str) -> Option<&'a str> {
    name.strip_suffix(ext)?.strip_suffix('.').filter(|stem| !stem.is_empty())
}

impl ScanTree {
    pub fn new(root_path: &str, root_name: &str, root_flags: u8, cleanup_ids: Vec<&'static str>) -> Self {
        let mut tree = Self {
            dirs: ChunkedVec::new(),
            files: ChunkedVec::new(),
            names: NamePool::new(),
            exts: Vec::new(),
            ext_index: HashMap::new(),
            cleanup_ids,
            root_path: root_path.to_string(),
            garbage: 0,
        };
        tree.push_dir(root_name, NO_NODE, root_flags);
        tree
    }

    pub fn root_path(&self) -> &str {
        &self.root_path
    }

    /// 전체 노드 수 (디렉토리 + 파일)
    pub fn len(&self) -> usize {
        self.dirs.len() + self.files.len()
    }

    pub fn dir_len(&self) -> usize {
        self.dirs.len()
    }

    #[inline]
    pub fn dir(&self, id: NodeId) -> &DirNode {
        debug_assert!(!is_file_id(id));
        &self.dirs[id as usize]
    }

    #[inline]
    pub fn dir_mut(&mut self, id: NodeId) -> &mut DirNode {
        debug_assert!(!is_file_id(id));
        &mut self.dirs[id as usize]
    }

    #[inline]
    pub fn file(&self, id: NodeId) -> &FileNode {
        &self.files[(id & !FILE_BIT) as usize]
    }

    #[inline]
    pub fn file_mut(&mut self, id: NodeId) -> &mut FileNode {
        &mut self.files[(id & !FILE_BIT) as usize]
    }

    #[inline]
    pub fn is_dir(&self, id: NodeId) -> bool {
        !is_file_id(id)
    }

    #[inline]
    pub fn flags(&self, id: NodeId) -> u8 {
        if is_file_id(id) { self.file(id).flags } else { self.dir(id).flags }
    }

    #[inline]
    pub fn has(&self, id: NodeId, flag: u8) -> bool {
        self.flags(id) & flag != 0
    }

    pub fn set_flag(&mut self, id: NodeId, flag: u8) {
        if is_file_id(id) { self.file_mut(id).flags |= flag } else { self.dir_mut(id).flags |= flag }
    }

    #[inline]
    pub fn size(&self, id: NodeId) -> u64 {
        if is_file_id(id) { self.file(id).size } else { self.dir(id).size }
    }

    pub fn file_count(&self, id: NodeId) -> u64 {
        if !is_file_id(id) {
            return self.dir(id).file_count;
        }
        let f = self.file(id);
        if f.flags & FLAG_OTHERS != 0 { f.ext as u64 } else { 1 }
    }

    pub fn modified(&self, id: NodeId) -> u64 {
        if is_file_id(id) { self.file(id).modified as u64 } else { self.dir(id).modified }
    }

    pub fn push_dir(&mut self, name: &str, parent: NodeId, flags: u8) -> NodeId {
        let (name_off, name_len) = self.push_name(name);
        let id = self.dirs.len() as NodeId;
        self.dirs.push(DirNode {
            size: 0,
            file_count: 0,
            modified: 0,
            parent,
            name_off,
            dirs_start: 0,
            dirs_len: 0,
            files_start: 0,
            files_len: 0,
            dir_count: 0,
            name_len,
            cleanup: 0,
            flags: flags | FLAG_DIR,
        });
        id
    }

    /// 파일 노드 추가. ext가 주어지고 이름이 ".{ext}"로 끝나면 stem만 이름 풀에 저장한다.
    pub fn push_file(&mut self, name: &str, ext: Option<&str>, flags: u8) -> NodeId {
        let mut flags = flags & !(FLAG_DIR | FLAG_EXT_STRIPPED);
        let mut ext_id = 0;
        let mut stored = name;
        if let Some(e) = ext {
            ext_id = self.intern_ext(e);
            if let Some(stem) = strip_ext(name, e) {
                stored = stem;
                flags |= FLAG_EXT_STRIPPED;
            }
        }
        let (name_off, name_len) = self.push_name(stored);
        let id = file_id(self.files.len() as u32);
        self.files.push(FileNode { size: 0, name_off, modified: 0, ext: ext_id, name_len, flags });
        id
    }

    pub(crate) fn push_name(&mut self, name: &str) -> (u32, u16) {
        let bytes = truncate_utf8(name, u16::MAX as usize);
        (self.names.push(bytes.as_bytes()), bytes.len() as u16)
    }

    /// 이름 풀에 저장된 그대로의 이름 (확장자가 떼어졌을 수 있음)
    pub(crate) fn raw_name(&self, id: NodeId) -> &str {
        let (off, len) = if is_file_id(id) {
            let f = self.file(id);
            (f.name_off, f.name_len)
        } else {
            let d = self.dir(id);
            (d.name_off, d.name_len)
        };
        std::str::from_utf8(self.names.get(off, len)).unwrap_or("")
    }

    /// 표시용 전체 이름
    pub fn name(&self, id: NodeId) -> Cow<'_, str> {
        let raw = self.raw_name(id);
        match self.stripped_ext(id) {
            Some(ext) => Cow::Owned(format!("{raw}.{ext}")),
            None => Cow::Borrowed(raw),
        }
    }

    fn stripped_ext(&self, id: NodeId) -> Option<&str> {
        if !is_file_id(id) || self.file(id).flags & FLAG_EXT_STRIPPED == 0 {
            return None;
        }
        self.ext(id)
    }

    /// 경로 세그먼트가 이 노드를 가리키는지 (할당 없이 비교)
    fn segment_matches(&self, id: NodeId, seg: &str) -> bool {
        if self.has(id, FLAG_OTHERS) {
            return seg == OTHERS_SEGMENT;
        }
        let raw = self.raw_name(id);
        match self.stripped_ext(id) {
            Some(ext) => strip_ext(seg, ext) == Some(raw),
            None => raw == seg,
        }
    }

    pub fn intern_ext(&mut self, ext: &str) -> u32 {
        if let Some(&i) = self.ext_index.get(ext) {
            return i + 1;
        }
        let i = self.exts.len() as u32;
        let boxed: Box<str> = ext.into();
        self.exts.push(boxed.clone());
        self.ext_index.insert(boxed, i);
        i + 1
    }

    pub fn ext(&self, id: NodeId) -> Option<&str> {
        if !is_file_id(id) {
            return None;
        }
        let f = self.file(id);
        if f.flags & FLAG_OTHERS != 0 || f.ext == 0 {
            return None;
        }
        self.exts.get(f.ext as usize - 1).map(|s| s.as_ref())
    }

    pub fn cleanup_id(&self, id: NodeId) -> Option<&'static str> {
        if is_file_id(id) {
            return None;
        }
        match self.dir(id).cleanup {
            0 => None,
            c => self.cleanup_ids.get(c as usize - 1).copied(),
        }
    }

    /// 삭제되지 않은 자식 (디렉토리 먼저, 이후 파일). 파일 id에 대해서는 비어 있다.
    pub fn children(&self, id: NodeId) -> impl Iterator<Item = NodeId> + '_ {
        let (dirs, files) = if is_file_id(id) {
            (0..0, 0..0)
        } else {
            let d = self.dir(id);
            (d.dirs_start..d.dirs_start + d.dirs_len, d.files_start..d.files_start + d.files_len)
        };
        dirs.filter(move |&c| self.dirs[c as usize].flags & FLAG_REMOVED == 0)
            .chain(files.filter(move |&c| self.files[c as usize].flags & FLAG_REMOVED == 0).map(file_id))
    }

    pub fn child_count(&self, id: NodeId) -> u32 {
        self.children(id).count() as u32
    }

    /// 루트부터 디렉토리 dir까지의 절대 경로
    pub fn path_of(&self, dir: NodeId) -> String {
        let mut chain = Vec::new();
        let mut cur = dir;
        while cur != 0 && cur != NO_NODE {
            chain.push(cur);
            cur = self.dir(cur).parent;
        }
        let mut path = self.root_path.clone();
        for &c in chain.iter().rev() {
            push_segment(&mut path, self.raw_name(c));
        }
        path
    }

    pub fn child_path(&self, parent_path: &str, child: NodeId) -> String {
        let mut path = parent_path.to_string();
        if self.has(child, FLAG_OTHERS) {
            push_segment(&mut path, OTHERS_SEGMENT);
        } else {
            push_segment(&mut path, &self.name(child));
        }
        path
    }

    /// (id, 부모 디렉토리)의 정규화된 절대 경로
    pub fn entry_path(&self, id: NodeId, parent: NodeId) -> String {
        if is_file_id(id) { self.child_path(&self.path_of(parent), id) } else { self.path_of(id) }
    }

    /// 절대 경로 → (NodeId, 부모 디렉토리 id). 루트의 부모는 NO_NODE.
    pub fn resolve_entry(&self, path: &str) -> Option<(NodeId, NodeId)> {
        let root = self.root_path.trim_end_matches('/');
        let path_trimmed = path.trim_end_matches('/');
        if path_trimmed == root {
            return Some((0, NO_NODE));
        }
        let rest = path_trimmed.strip_prefix(root)?;
        if !rest.starts_with('/') {
            return None;
        }
        let (mut cur, mut parent) = (0, NO_NODE);
        for seg in rest.split('/').filter(|s| !s.is_empty()) {
            let next = self.children(cur).find(|&c| self.segment_matches(c, seg))?;
            parent = cur;
            cur = next;
        }
        Some((cur, parent))
    }

    pub fn resolve(&self, path: &str) -> Option<NodeId> {
        self.resolve_entry(path).map(|(id, _)| id)
    }

    /// 디렉토리 id 자신과 모든 조상에 크기/파일 수를 더한다.
    pub fn add_to_ancestors(&mut self, dir: NodeId, size: u64, files: u64, modified: u64) {
        let mut cur = dir;
        while cur != NO_NODE {
            let n = self.dir_mut(cur);
            n.size += size;
            n.file_count += files;
            if modified > n.modified {
                n.modified = modified;
            }
            cur = n.parent;
        }
    }

    /// 하위 디렉토리 수 집계. 자식 id는 항상 부모 id보다 크므로 역순 순회로 bottom-up 계산된다.
    pub fn compute_dir_counts(&mut self) {
        for i in (1..self.dirs.len()).rev() {
            let d = self.dirs[i];
            if d.flags & FLAG_REMOVED == 0 && d.parent != NO_NODE {
                self.dirs[d.parent as usize].dir_count += 1 + d.dir_count;
            }
        }
    }

    /// 서브트리(자신 포함) 노드 수
    pub fn subtree_len(&self, id: NodeId) -> usize {
        if is_file_id(id) {
            return 1;
        }
        let mut count = 0;
        let mut stack = vec![id];
        while let Some(cur) = stack.pop() {
            let d = self.dir(cur);
            count += 1 + d.files_len as usize;
            stack.extend(d.dirs_start..d.dirs_start + d.dirs_len);
        }
        count
    }

    pub fn shrink_to_fit(&mut self) {
        self.dirs.shrink_to_fit();
        self.files.shrink_to_fit();
        self.names.shrink_to_fit();
    }
}

pub fn push_segment(path: &mut String, seg: &str) {
    if !path.ends_with('/') {
        path.push('/');
    }
    path.push_str(seg);
}

fn truncate_utf8(s: &str, max: usize) -> &str {
    if s.len() <= max {
        return s;
    }
    let mut end = max;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

impl ScanTree {
    /// 구성 요소별 메모리 사용량 (벤치마크/진단용)
    pub fn memory_report(&self) -> String {
        const MB: usize = 1024 * 1024;
        let dirs = self.dirs.len() * std::mem::size_of::<DirNode>();
        let files = self.files.len() * std::mem::size_of::<FileNode>();
        let names = self.names.byte_len();
        format!(
            "arena: dirs={} MB files={} MB names={} MB total={} MB",
            dirs / MB,
            files / MB,
            names / MB,
            (dirs + files + names) / MB
        )
    }
}
