use std::fs;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::RwLock;

use crate::models::ScanProgress;
use crate::scanner::{scan_into, ScanOptions, ScanReporter};
use crate::tree::ScanTree;

struct NoopReporter;

impl ScanReporter for NoopReporter {
    fn progress(&self, _: ScanProgress) {}
    fn log(&self, _: &str, _: &str, _: &str) {}
}

fn options(max_files: usize) -> ScanOptions {
    ScanOptions {
        exclude_paths: Vec::new(),
        exclude_names: Vec::new(),
        follow_symlinks: false,
        max_children_per_dir: max_files,
        cross_device: false,
    }
}

fn write(path: &Path, bytes: usize) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, vec![b'x'; bytes]).unwrap();
}

fn scan(root: &Path, max_files: usize) -> ScanTree {
    let slot = RwLock::new(None);
    let ok = scan_into(root.to_str().unwrap(), &AtomicBool::new(false), &NoopReporter, &options(max_files), &slot);
    assert!(ok);
    slot.into_inner().unwrap().unwrap()
}

/// root/
///   a/ big.bin(64K) f0..f9.txt(4K each)
///   a/deep/ x.rs(8K)
///   b/ y.md(4K)
///   top.bin(16K)
fn fixture() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let r = dir.path();
    write(&r.join("a/big.bin"), 64 * 1024);
    for i in 0..10 {
        write(&r.join(format!("a/f{i}.txt")), 4 * 1024);
    }
    write(&r.join("a/deep/x.rs"), 8 * 1024);
    write(&r.join("b/y.md"), 4 * 1024);
    write(&r.join("top.bin"), 16 * 1024);
    dir
}

#[test]
fn aggregates_sizes_and_counts() {
    let dir = fixture();
    let tree = scan(dir.path(), 200);
    let root = tree.root_dto();
    assert_eq!(root.file_count, 14);
    assert_eq!(root.dir_count, 3);
    assert!(root.size >= (64 + 40 + 8 + 4 + 16) * 1024);

    let a = tree.node_dto(&format!("{}/a", tree.root_path())).unwrap();
    assert_eq!(a.file_count, 12);
    assert_eq!(a.dir_count, 1);
    assert_eq!(a.child_count, 12); // deep + 11 files
}

#[test]
fn prunes_files_beyond_limit_into_others_node() {
    let dir = fixture();
    let tree = scan(dir.path(), 3);
    let a_path = format!("{}/a", tree.root_path());
    let children = tree.children_dtos(&a_path).unwrap();
    // deep + top 3 files + others
    assert_eq!(children.len(), 5);
    assert_eq!(children[1].name, "big.bin");
    let others = children.last().unwrap();
    assert_eq!(others.path, format!("{a_path}/__others__"));
    assert_eq!(others.file_count, 8);
    // 합산값은 잘린 파일까지 포함해야 한다
    assert_eq!(tree.node_dto(&a_path).unwrap().file_count, 12);
    assert!(tree.resolve(&others.path).is_some());
}

#[test]
fn resolves_paths_and_builds_child_paths() {
    let dir = fixture();
    let tree = scan(dir.path(), 200);
    let deep = format!("{}/a/deep/x.rs", tree.root_path());
    let id = tree.resolve(&deep).unwrap();
    assert_eq!(tree.path_of(id), deep);
    assert!(tree.resolve(&format!("{}/missing", tree.root_path())).is_none());
    assert!(tree.resolve("/somewhere/else").is_none());
    assert_eq!(tree.resolve(&format!("{}/", tree.root_path())), Some(0));
}

#[test]
fn remove_updates_ancestors_and_hides_node() {
    let dir = fixture();
    let mut tree = scan(dir.path(), 200);
    let before = tree.root_dto();
    let a_path = format!("{}/a", tree.root_path());
    let a = tree.node_dto(&a_path).unwrap();

    assert!(tree.remove(tree.resolve(&a_path).unwrap()));
    let after = tree.root_dto();
    assert_eq!(after.size, before.size - a.size);
    assert_eq!(after.file_count, before.file_count - a.file_count);
    assert_eq!(after.dir_count, before.dir_count - 2);
    assert!(tree.resolve(&a_path).is_none());
    assert!(tree.children_dtos(tree.root_path()).unwrap().iter().all(|c| c.name != "a"));
}

#[test]
fn compact_preserves_live_tree() {
    let dir = fixture();
    let mut tree = scan(dir.path(), 200);
    let b = tree.resolve(&format!("{}/b", tree.root_path())).unwrap();
    tree.remove(b);
    let before = tree.root_dto();
    let len_before = tree.len();
    tree.compact();
    assert!(tree.len() < len_before);
    let after = tree.root_dto();
    assert_eq!(after.size, before.size);
    assert_eq!(after.child_count, before.child_count);
    let deep = format!("{}/a/deep/x.rs", tree.root_path());
    assert_eq!(tree.path_of(tree.resolve(&deep).unwrap()), deep);
}

#[test]
fn graft_replaces_subtree_and_propagates_delta() {
    let dir = fixture();
    let mut tree = scan(dir.path(), 200);
    let before = tree.root_dto();

    write(&dir.path().join("b/new.bin"), 32 * 1024);
    let b_path = format!("{}/b", tree.root_path());
    let sub = scan(Path::new(&b_path), 200);
    let sub_size = sub.root_dto().size;
    let old_b = tree.node_dto(&b_path).unwrap();

    tree.graft(tree.resolve(&b_path).unwrap(), &sub);
    let b = tree.node_dto(&b_path).unwrap();
    assert_eq!(b.size, sub_size);
    assert_eq!(b.file_count, 2);
    assert_eq!(tree.root_dto().size, before.size - old_b.size + sub_size);
    assert_eq!(tree.root_dto().file_count, before.file_count + 1);
    assert!(tree.resolve(&format!("{b_path}/new.bin")).is_some());
}

#[test]
fn treemap_limits_nodes_and_keeps_total_size() {
    let dir = fixture();
    let tree = scan(dir.path(), 200);
    let full = tree.treemap(tree.root_path(), 0.0, 200, 10_000).unwrap();
    assert_eq!(full.size, tree.root_dto().size);
    assert_eq!(full.children.iter().map(|c| c.size).sum::<u64>(), full.size);

    // 노드 예산을 넘으면 더 이상 확장하지 않는다
    let small = tree.treemap(tree.root_path(), 0.0, 200, 2).unwrap();
    let count = |n: &crate::models::TreemapNode| -> usize {
        let mut stack = vec![n];
        let mut c = 0;
        while let Some(x) = stack.pop() {
            c += 1;
            stack.extend(x.children.iter());
        }
        c
    };
    assert!(count(&small) < count(&full));

    // min_ratio로 잘린 자식들은 others 노드로 합산된다
    let pruned = tree.treemap(tree.root_path(), 0.3, 200, 10_000).unwrap();
    assert_eq!(pruned.children.iter().map(|c| c.size).sum::<u64>(), pruned.size);
    assert!(pruned.children.iter().any(|c| c.path.ends_with("/__others__")));
}

#[test]
fn extension_stats_cover_all_files() {
    let dir = fixture();
    let tree = scan(dir.path(), 3);
    let stats = tree.extension_stats(tree.root_path());
    let files: u64 = stats.iter().map(|s| s.file_count).sum();
    assert_eq!(files, 14);
    assert_eq!(stats.iter().map(|s| s.total_bytes).sum::<u64>(), tree.root_dto().size);
}

/// 실제 홈 디렉토리 스캔 시 최대 RSS 측정:
/// `cargo test --release --lib scan_home_memory -- --ignored --nocapture`
#[test]
#[ignore]
fn scan_home_memory() {
    let home = std::env::var("HOME").unwrap();
    let start = std::time::Instant::now();
    let tree = scan(Path::new(&home), 200);
    let root = tree.root_dto();
    let mut usage: libc::rusage = unsafe { std::mem::zeroed() };
    unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut usage) };
    // macOS: ru_maxrss는 bytes 단위
    println!(
        "files={} dirs={} nodes={} size={} elapsed={:?} peak_rss={} MB",
        root.file_count,
        root.dir_count,
        tree.len(),
        root.size,
        start.elapsed(),
        usage.ru_maxrss / 1024 / 1024
    );
}
