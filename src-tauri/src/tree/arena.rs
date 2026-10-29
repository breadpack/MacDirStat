use std::collections::HashMap;

pub type NodeId = u32;
pub const NO_NODE: NodeId = u32::MAX;

pub const FLAG_DIR: u8 = 1;
pub const FLAG_SYMLINK: u8 = 2;
pub const FLAG_HIDDEN: u8 = 4;
pub const FLAG_READONLY: u8 = 8;
/// 디렉토리당 상한을 넘어 개별 노드로 저장하지 않은 파일들의 합산 노드
pub const FLAG_OTHERS: u8 = 16;
pub const FLAG_REMOVED: u8 = 32;

/// 경로 세그먼트로 사용되는 합산 노드의 가상 이름
pub const OTHERS_SEGMENT: &str = "__others__";

/// 64바이트 고정 크기 노드. 경로는 저장하지 않고 parent 체인으로 계산한다.
/// 자식은 arena 내 연속 구간(dirs / files)으로 표현하여 노드별 Vec 할당을 없앤다.
#[derive(Clone, Copy, Debug)]
pub struct Node {
    pub size: u64,
    pub file_count: u64,
    /// unix seconds, 0 = unknown
    pub modified: u64,
    pub parent: NodeId,
    pub name_off: u32,
    pub dirs_start: NodeId,
    pub dirs_len: u32,
    pub files_start: NodeId,
    pub files_len: u32,
    /// 하위 디렉토리 총 개수
    pub dir_count: u32,
    /// 0 = 없음, 그 외 exts 인덱스 + 1
    pub ext: u32,
    pub name_len: u16,
    /// 0 = 없음, 그 외 cleanup pattern 인덱스 + 1
    pub cleanup: u16,
    pub flags: u8,
}

impl Node {
    fn new(parent: NodeId, name_off: u32, name_len: u16, flags: u8) -> Self {
        Self {
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
            ext: 0,
            name_len,
            cleanup: 0,
            flags,
        }
    }

    #[inline]
    pub fn has(&self, flag: u8) -> bool {
        self.flags & flag != 0
    }

    #[inline]
    pub fn is_dir(&self) -> bool {
        self.has(FLAG_DIR)
    }
}

/// 스캔 결과 전체를 담는 arena. 노드 0이 루트.
pub struct ScanTree {
    pub(crate) nodes: Vec<Node>,
    pub(crate) names: Vec<u8>,
    pub(crate) exts: Vec<Box<str>>,
    ext_index: HashMap<Box<str>, u32>,
    pub(crate) cleanup_ids: Vec<&'static str>,
    pub(crate) root_path: String,
    /// 더 이상 도달할 수 없는 노드 수 (삭제/서브트리 교체로 발생)
    pub(crate) garbage: usize,
}

impl ScanTree {
    pub fn new(root_path: &str, root_name: &str, root_flags: u8, cleanup_ids: Vec<&'static str>) -> Self {
        let mut tree = Self {
            nodes: Vec::new(),
            names: Vec::new(),
            exts: Vec::new(),
            ext_index: HashMap::new(),
            cleanup_ids,
            root_path: root_path.to_string(),
            garbage: 0,
        };
        tree.push_node(root_name, NO_NODE, root_flags | FLAG_DIR);
        tree
    }

    pub fn root_path(&self) -> &str {
        &self.root_path
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    #[inline]
    pub fn node(&self, id: NodeId) -> &Node {
        &self.nodes[id as usize]
    }

    #[inline]
    pub fn node_mut(&mut self, id: NodeId) -> &mut Node {
        &mut self.nodes[id as usize]
    }

    pub fn push_node(&mut self, name: &str, parent: NodeId, flags: u8) -> NodeId {
        let (off, len) = self.push_name(name);
        let id = self.nodes.len() as NodeId;
        self.nodes.push(Node::new(parent, off, len, flags));
        id
    }

    pub(crate) fn push_name(&mut self, name: &str) -> (u32, u16) {
        let bytes = truncate_utf8(name, u16::MAX as usize);
        let off = self.names.len() as u32;
        self.names.extend_from_slice(bytes.as_bytes());
        (off, bytes.len() as u16)
    }

    pub fn name(&self, id: NodeId) -> &str {
        let n = self.node(id);
        let start = n.name_off as usize;
        std::str::from_utf8(&self.names[start..start + n.name_len as usize]).unwrap_or("")
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
        match self.node(id).ext {
            0 => None,
            e => self.exts.get(e as usize - 1).map(|s| s.as_ref()),
        }
    }

    pub fn cleanup_id(&self, id: NodeId) -> Option<&'static str> {
        match self.node(id).cleanup {
            0 => None,
            c => self.cleanup_ids.get(c as usize - 1).copied(),
        }
    }

    /// 삭제되지 않은 자식 (디렉토리 먼저, 이후 파일)
    pub fn children(&self, id: NodeId) -> impl Iterator<Item = NodeId> + '_ {
        let n = self.node(id);
        let dirs = n.dirs_start..n.dirs_start + n.dirs_len;
        let files = n.files_start..n.files_start + n.files_len;
        dirs.chain(files).filter(move |&c| !self.node(c).has(FLAG_REMOVED))
    }

    pub fn child_count(&self, id: NodeId) -> u32 {
        self.children(id).count() as u32
    }

    /// 루트부터 id까지의 절대 경로
    pub fn path_of(&self, id: NodeId) -> String {
        let mut chain = Vec::new();
        let mut cur = id;
        while cur != 0 && cur != NO_NODE {
            chain.push(cur);
            cur = self.node(cur).parent;
        }
        let mut path = self.root_path.clone();
        for &c in chain.iter().rev() {
            let seg = if self.node(c).has(FLAG_OTHERS) { OTHERS_SEGMENT } else { self.name(c) };
            push_segment(&mut path, seg);
        }
        path
    }

    pub fn child_path(&self, parent_path: &str, child: NodeId) -> String {
        let mut path = parent_path.to_string();
        let seg = if self.node(child).has(FLAG_OTHERS) { OTHERS_SEGMENT } else { self.name(child) };
        push_segment(&mut path, seg);
        path
    }

    /// 절대 경로 → NodeId. 루트 밖이거나 존재하지 않으면 None.
    pub fn resolve(&self, path: &str) -> Option<NodeId> {
        let root = self.root_path.trim_end_matches('/');
        let path_trimmed = path.trim_end_matches('/');
        if path_trimmed == root {
            return Some(0);
        }
        let rest = path_trimmed.strip_prefix(root)?;
        if !rest.starts_with('/') {
            return None;
        }
        let mut cur: NodeId = 0;
        for seg in rest.split('/').filter(|s| !s.is_empty()) {
            cur = self.children(cur).find(|&c| {
                let n = self.node(c);
                if n.has(FLAG_OTHERS) { seg == OTHERS_SEGMENT } else { self.name(c) == seg }
            })?;
        }
        Some(cur)
    }

    /// id 자신과 모든 조상에 크기/파일 수를 더한다.
    pub fn add_to_ancestors(&mut self, id: NodeId, size: u64, files: u64, modified: u64) {
        let mut cur = id;
        while cur != NO_NODE {
            let n = self.node_mut(cur);
            n.size += size;
            n.file_count += files;
            if modified > n.modified {
                n.modified = modified;
            }
            cur = n.parent;
        }
    }

    /// 루트를 제외한 노드 중 디렉토리만 있는 상태(skeleton)에서 하위 디렉토리 수 집계.
    /// 자식 id는 항상 부모 id보다 크므로 역순 순회로 bottom-up 계산된다.
    pub fn compute_dir_counts(&mut self) {
        for i in (1..self.nodes.len()).rev() {
            let n = self.nodes[i];
            if n.is_dir() && !n.has(FLAG_REMOVED) && n.parent != NO_NODE {
                self.nodes[n.parent as usize].dir_count += 1 + n.dir_count;
            }
        }
    }

    /// 서브트리(자신 포함) 노드 수
    pub fn subtree_len(&self, id: NodeId) -> usize {
        let mut count = 0;
        let mut stack = vec![id];
        while let Some(cur) = stack.pop() {
            count += 1;
            let n = self.node(cur);
            stack.extend(n.dirs_start..n.dirs_start + n.dirs_len);
            count += n.files_len as usize;
        }
        count
    }

    pub fn shrink_to_fit(&mut self) {
        self.nodes.shrink_to_fit();
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
