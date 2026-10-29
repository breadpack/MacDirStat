use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{sync_channel, RecvTimeoutError};
use std::sync::RwLock;
use std::time::{Duration, Instant};

use dashmap::DashSet;
use rayon::prelude::*;

use crate::models::ScanProgress;
use crate::platform;
use crate::scanner_files::scan_dir_files;
use crate::skeleton;
use crate::tree::{DirResult, ScanTree};

pub const DEFAULT_SKIP_PATHS: &[&str] = &[
    "/dev", "/proc", "/sys", "/Volumes", "/.vol", "/net", "/home", "/Network",
    "/System/Volumes",
    "/private/var/folders",
    "/private/var/db",
    "/private/var/protected",
    "/private/var/audit",
    "/.Spotlight-V100",
    "/.fseventsd",
    "/.Trashes",
];

pub const DEFAULT_SKIP_NAMES: &[&str] = &[
    ".Spotlight-V100", ".fseventsd", ".Trashes", ".DocumentRevisions-V100",
];

/// 진행 상황 전송 주기 (프론트엔드는 tree_version 변경 시 재조회)
const PROGRESS_INTERVAL: Duration = Duration::from_millis(1000);
/// Phase 2 결과 채널 용량. 집계 스레드가 밀리면 워커가 대기하여 메모리 사용이 제한된다.
const RESULT_CHANNEL_CAP: usize = 1024;
/// write lock 한 번에 반영할 최대 결과 수
const APPLY_BATCH: usize = 2048;

/// Runtime scan options passed from the frontend settings.
#[derive(Clone)]
pub struct ScanOptions {
    pub exclude_paths: Vec<String>,
    pub exclude_names: Vec<String>,
    pub follow_symlinks: bool,
    /// 디렉토리당 개별 노드로 유지할 최대 파일 수. 나머지는 "(N other files)"로 합산된다.
    pub max_children_per_dir: usize,
    pub cross_device: bool,
}

/// 스캔 진행/로그 전달 대상 (Tauri Channel 또는 테스트용 구현)
pub trait ScanReporter: Sync {
    fn progress(&self, progress: ScanProgress);
    fn log(&self, level: &str, message: &str, path: &str);
}

pub(crate) fn make_progress(phase: &str) -> ScanProgress {
    ScanProgress {
        phase: phase.into(),
        files_scanned: 0,
        current_path: String::new(),
        total_dirs: 0,
        completed_dirs: 0,
        total_bytes: 0,
        tree_version: 0,
    }
}

pub(crate) fn write_lock<T>(lock: &RwLock<T>) -> std::sync::RwLockWriteGuard<'_, T> {
    lock.write().unwrap_or_else(|e| e.into_inner())
}

pub(crate) fn read_lock<T>(lock: &RwLock<T>) -> std::sync::RwLockReadGuard<'_, T> {
    lock.read().unwrap_or_else(|e| e.into_inner())
}

/// Phase 2 워커가 경로를 계산하기 위한 디렉토리 인덱스 (arena lock 없이 접근)
struct DirIndex {
    root: String,
    parent: Vec<u32>,
    name_off: Vec<u32>,
    name_len: Vec<u16>,
    names: Vec<u8>,
}

impl DirIndex {
    fn from_tree(tree: &ScanTree) -> Self {
        let n = tree.dir_len();
        let mut idx = DirIndex {
            root: tree.root_path().to_string(),
            parent: Vec::with_capacity(n),
            name_off: Vec::with_capacity(n),
            name_len: Vec::with_capacity(n),
            names: tree.names.clone(),
        };
        for node in &tree.dirs {
            idx.parent.push(node.parent);
            idx.name_off.push(node.name_off);
            idx.name_len.push(node.name_len);
        }
        idx
    }

    fn path(&self, id: u32) -> PathBuf {
        let mut chain = Vec::new();
        let mut cur = id;
        while cur != 0 {
            chain.push(cur);
            cur = self.parent[cur as usize];
        }
        let mut path = PathBuf::from(&self.root);
        for &c in chain.iter().rev() {
            let off = self.name_off[c as usize] as usize;
            let len = self.name_len[c as usize] as usize;
            path.push(std::str::from_utf8(&self.names[off..off + len]).unwrap_or(""));
        }
        path
    }
}

/// root를 스캔하여 결과 트리를 slot에 기록한다.
/// Phase 1 완료 직후 skeleton이 slot에 들어가고, Phase 2 동안 실시간으로 크기가 채워지므로
/// 다른 스레드가 slot을 읽어 진행 중인 트리를 조회할 수 있다.
/// 반환값: 완료되면 true, 취소/실패 시 false.
pub fn scan_into(
    root: &str,
    cancel_token: &AtomicBool,
    reporter: &dyn ScanReporter,
    options: &ScanOptions,
    slot: &RwLock<Option<ScanTree>>,
) -> bool {
    SKIP_LOG_COUNT.store(0, Ordering::Relaxed);

    let root_path = Path::new(root);
    if !root_path.exists() {
        send_log(reporter, "error", &format!("Path does not exist: {}", root), root);
        return false;
    }

    // Get the device ID of the root path for mount-point boundary detection
    #[cfg(unix)]
    let root_dev = {
        use std::os::unix::fs::MetadataExt;
        std::fs::metadata(root_path).ok().map(|m| m.dev())
    };
    #[cfg(not(unix))]
    let root_dev: Option<u64> = None;

    // ── Phase 1: 디렉토리 구조만 빠르게 스캔 ──
    reporter.progress(ScanProgress { current_path: root.to_string(), ..make_progress("structure") });
    send_log(reporter, "info", "Phase 1: Scanning directory structure...", root);

    let patterns = crate::cleanup_patterns::all_patterns();
    let scan_map = crate::cleanup_patterns::build_scan_pattern_map(&patterns);
    let skeleton = skeleton::build_skeleton(root_path, cancel_token, reporter, options, root_dev, &scan_map, &patterns);

    if cancel_token.load(Ordering::Relaxed) {
        send_log(reporter, "info", "Scan cancelled by user", root);
        return false;
    }
    let Some(skeleton) = skeleton else {
        send_log(reporter, "error", "Failed to scan directory structure", root);
        return false;
    };

    let total_dirs = skeleton.dir_len() as u32;
    send_log(reporter, "info", &format!("Phase 1 complete: {} directories found", total_dirs), root);
    let index = DirIndex::from_tree(&skeleton);
    *write_lock(slot) = Some(skeleton);

    // ── Phase 2: 모든 디렉토리의 파일 크기를 병렬 스캔, 결과는 단일 집계 루프가 트리에 반영 ──
    let mut version: u64 = 1;
    reporter.progress(ScanProgress {
        current_path: root.to_string(),
        total_dirs,
        tree_version: version,
        ..make_progress("files")
    });
    send_log(reporter, "info", &format!("Phase 2: Scanning files in {} directories (parallel)...", total_dirs), root);

    let seen_inodes: DashSet<u64> = DashSet::new();
    let max_files = options.max_children_per_dir;
    let (tx, rx) = sync_channel::<DirResult>(RESULT_CHANNEL_CAP);
    let total_files = AtomicU64::new(0);

    std::thread::scope(|scope| {
        let index = &index;
        let seen_inodes = &seen_inodes;
        scope.spawn(move || {
            (0..total_dirs).into_par_iter().for_each_with(tx, |tx, id| {
                if cancel_token.load(Ordering::Relaxed) {
                    return;
                }
                let r = scan_dir_files(id, &index.path(id), max_files, seen_inodes, reporter);
                let _ = tx.send(r);
            });
        });

        let mut completed: u32 = 0;
        let mut total_bytes: u64 = 0;
        let mut dirty = false;
        let mut last_emit = Instant::now();
        loop {
            match rx.recv_timeout(Duration::from_millis(200)) {
                Ok(first) => {
                    let mut guard = write_lock(slot);
                    let Some(tree) = guard.as_mut() else { break };
                    let mut r = Some(first);
                    let mut applied = 0;
                    while let Some(res) = r.take() {
                        completed += 1;
                        total_bytes += res.bytes;
                        total_files.fetch_add(res.count, Ordering::Relaxed);
                        tree.apply_dir_result(res);
                        applied += 1;
                        if applied < APPLY_BATCH {
                            r = rx.try_recv().ok();
                        }
                    }
                    dirty = true;
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => break,
            }
            if dirty && last_emit.elapsed() >= PROGRESS_INTERVAL {
                version += 1;
                dirty = false;
                last_emit = Instant::now();
                reporter.progress(ScanProgress {
                    phase: "files".into(),
                    files_scanned: total_files.load(Ordering::Relaxed),
                    current_path: String::new(),
                    total_dirs,
                    completed_dirs: completed,
                    total_bytes,
                    tree_version: version,
                });
            }
        }
        // 집계 루프가 중단되어도 워커가 send에서 막히지 않도록 수신측을 먼저 닫는다
        drop(rx);
    });

    if cancel_token.load(Ordering::Relaxed) {
        send_log(reporter, "info", "Scan cancelled by user", root);
        return false;
    }

    let mut guard = write_lock(slot);
    let Some(tree) = guard.as_mut() else { return false };
    tree.shrink_to_fit();
    let root_node = tree.dir(0);
    send_log(
        reporter,
        "info",
        &format!("Scan complete: {} files, {} bytes, {} nodes in memory", root_node.file_count, root_node.size, tree.len()),
        root,
    );
    true
}

pub(crate) fn should_skip_dynamic(path: &Path, exclude_paths: &[String], exclude_names: &[String]) -> bool {
    let path_str = path.to_string_lossy();
    if exclude_paths.iter().any(|skip| path_str.as_ref() == skip.as_str()) { return true; }
    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
        if exclude_names.iter().any(|skip| name == skip.as_str()) { return true; }
    }
    false
}

pub(crate) fn is_ignorable_error(err_msg: &str, path: &str) -> bool {
    platform::is_system_protected(path)
        || err_msg.contains("File name too long")
        || err_msg.contains("No such file or directory")
        || err_msg.contains("Operation not permitted")
        || err_msg.contains("Permission denied")
}

static SKIP_LOG_COUNT: AtomicU64 = AtomicU64::new(0);

pub(crate) fn send_log(reporter: &dyn ScanReporter, level: &str, message: &str, path: &str) {
    if level == "skip" {
        let count = SKIP_LOG_COUNT.fetch_add(1, Ordering::Relaxed);
        if count % 100 != 0 { return; }
    }
    reporter.log(level, message, path);
}
