use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::UNIX_EPOCH;

use dashmap::{DashMap, DashSet};
use rayon::prelude::*;
use tauri::ipc::Channel;

use crate::models::{FileNode, ScanLogEntry, ScanProgress};
use crate::platform;
use crate::scanner_tree;

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

/// Runtime scan options passed from the frontend settings.
#[derive(Clone)]
pub struct ScanOptions {
    pub exclude_paths: Vec<String>,
    pub exclude_names: Vec<String>,
    pub follow_symlinks: bool,
    pub max_children_per_dir: usize,
    pub cross_device: bool,
}

pub(crate) fn make_progress(phase: &str) -> ScanProgress {
    ScanProgress {
        phase: phase.into(),
        files_scanned: 0,
        current_path: String::new(),
        total_dirs: 0,
        completed_dirs: 0,
        current_dir_name: String::new(),
        current_dir_files: 0,
        current_dir_bytes: 0,
        total_bytes: 0,
        scanning_dirs: Vec::new(),
        dir_sizes: Vec::new(),
    }
}

pub fn scan_directory(
    root: &str,
    cancel_token: Arc<AtomicBool>,
    on_progress: Channel<ScanProgress>,
    on_log: Channel<ScanLogEntry>,
    on_tree: Channel<FileNode>,
    options: ScanOptions,
) -> Option<FileNode> {
    // Reset skip log count at scan start
    SKIP_LOG_COUNT.store(0, Ordering::Relaxed);

    let root_path = Path::new(root);
    if !root_path.exists() {
        send_log(&on_log, "error", &format!("Path does not exist: {}", root), root);
        return None;
    }

    // Get the device ID of the root path for mount-point boundary detection (Fix 5)
    #[cfg(unix)]
    let root_dev = {
        use std::os::unix::fs::MetadataExt;
        std::fs::metadata(root_path).ok().map(|m| m.dev())
    };
    #[cfg(not(unix))]
    let root_dev: Option<u64> = None;

    // ── Phase 1: 디렉토리 구조만 빠르게 스캔 ──
    let _ = on_progress.send(ScanProgress {
        current_path: root.to_string(),
        ..make_progress("structure")
    });

    send_log(&on_log, "info", "Phase 1: Scanning directory structure...", root);

    let mut dir_count: u32 = 0;
    let skeleton = scanner_tree::build_skeleton(
        root_path, &cancel_token, &on_progress, &on_log, &mut dir_count, &options, root_dev,
    );

    if cancel_token.load(Ordering::Relaxed) {
        send_log(&on_log, "info", "Scan cancelled by user", root);
        return None;
    }

    let skeleton = match skeleton {
        Some(s) => s,
        None => {
            send_log(&on_log, "error", "Failed to scan directory structure", root);
            return None;
        }
    };

    send_log(&on_log, "info", &format!("Phase 1 complete: {} directories found", dir_count), root);
    let _ = on_tree.send(skeleton.clone());

    // ── Phase 2: 모든 디렉토리의 파일 크기를 병렬 스캔 ──
    let mut all_dirs = Vec::new();
    scanner_tree::collect_all_dir_paths(&skeleton, &mut all_dirs);
    let total_dirs = all_dirs.len() as u32;

    let completed_dirs = Arc::new(AtomicU32::new(0));
    let total_files = Arc::new(AtomicU64::new(0));
    let total_bytes = Arc::new(AtomicU64::new(0));
    let dir_sizes_map: Arc<DashMap<String, (u64, u64)>> = Arc::new(DashMap::new());
    let file_results: Arc<DashMap<String, Vec<FileNode>>> = Arc::new(DashMap::new());
    // Track seen inodes to avoid counting hardlinked files multiple times
    let seen_inodes: Arc<DashSet<u64>> = Arc::new(DashSet::new());

    let _ = on_progress.send(ScanProgress {
        current_path: root.to_string(),
        total_dirs,
        ..make_progress("files")
    });

    send_log(&on_log, "info", &format!("Phase 2: Scanning files in {} directories (parallel)...", total_dirs), root);

    // 1초마다 progress 전송하는 타이머 스레드 (delta 방식: 마지막 전송 이후 변경분만)
    let timer_running = Arc::new(AtomicBool::new(true));
    let timer_thread = {
        let running = timer_running.clone();
        let cancel = cancel_token.clone();
        let dsm = dir_sizes_map.clone();
        let tfs = total_files.clone();
        let tb = total_bytes.clone();
        let cd = completed_dirs.clone();
        let prog = on_progress.clone();
        std::thread::spawn(move || {
            let mut last_sent_count: usize = 0;
            while running.load(Ordering::Relaxed) && !cancel.load(Ordering::Relaxed) {
                std::thread::sleep(std::time::Duration::from_secs(1));
                if !running.load(Ordering::Relaxed) { break; }

                // 변경분만 추출 (최대 500개로 제한하여 채널 과부하 방지)
                let current_count = dsm.len();
                let sizes: Vec<(String, u64, u64)> = if current_count == last_sent_count {
                    Vec::new()
                } else {
                    let delta: Vec<(String, u64, u64)> = dsm.iter()
                        .skip(last_sent_count)
                        .take(500)
                        .map(|entry| (entry.key().clone(), entry.value().0, entry.value().1))
                        .collect();
                    last_sent_count = std::cmp::min(last_sent_count + 500, current_count);
                    delta
                };

                let _ = prog.send(ScanProgress {
                    phase: "files".into(),
                    files_scanned: tfs.load(Ordering::Relaxed),
                    current_path: String::new(),
                    total_dirs,
                    completed_dirs: cd.load(Ordering::Relaxed),
                    total_bytes: tb.load(Ordering::Relaxed),
                    scanning_dirs: Vec::new(),
                    dir_sizes: sizes,
                    ..make_progress("files")
                });
            }
        })
    };

    // 모든 디렉토리를 병렬로 스캔 (각 디렉토리 내 파일의 크기를 읽음)
    all_dirs.par_iter().for_each(|dir_path| {
        if cancel_token.load(Ordering::Relaxed) { return; }

        let mut files = Vec::new();
        let mut bytes = 0u64;
        let mut count = 0u64;

        match std::fs::read_dir(dir_path) {
            Ok(entries) => {
                for entry in entries.flatten() {
                    let ft = match entry.file_type() { Ok(ft) => ft, Err(_) => continue };
                    if !ft.is_file() { continue; }

                    let path = entry.path();
                    let (size, modified, is_readonly, nlink, ino) = match entry.metadata() {
                        Ok(m) => {
                            let mod_time = m.modified().ok().and_then(|t| {
                                t.duration_since(UNIX_EPOCH).ok().map(|d| d.as_secs())
                            });
                            #[cfg(unix)]
                            let (physical_size, readonly, nlink, ino) = {
                                use std::os::unix::fs::MetadataExt;
                                use std::os::unix::fs::PermissionsExt;
                                // Fix 1: Use physical disk usage (blocks * 512) instead of logical size
                                let blocks = m.blocks();
                                let phys = if blocks > 0 { blocks as u64 * 512 } else { m.len() };
                                (phys, m.permissions().mode() & 0o222 == 0, m.nlink(), m.ino())
                            };
                            #[cfg(not(unix))]
                            let (physical_size, readonly, nlink, ino) = (m.len(), m.permissions().readonly(), 1u64, 0u64);
                            (physical_size, mod_time, readonly, nlink, ino)
                        }
                        Err(e) => {
                            let ps = path.to_string_lossy().to_string();
                            let msg = e.to_string();
                            let level = if is_ignorable_error(&msg, &ps) { "skip" } else { "warn" };
                            send_log(&on_log, level, &format!("Cannot read metadata: {}", msg), &ps);
                            (0, None, false, 1, 0)
                        }
                    };

                    // Skip hardlinked files already counted (nlink > 1 means multiple hardlinks)
                    if nlink > 1 && ino != 0 {
                        if !seen_inodes.insert(ino) {
                            // Already counted this inode, skip
                            continue;
                        }
                    }

                    let fname = entry.file_name().to_string_lossy().to_string();
                    let is_hidden = fname.starts_with('.');
                    let is_symlink = ft.is_symlink();

                    // Fix 2: iCloud placeholder detection
                    let (display_name, final_size) = if platform::is_icloud_placeholder(&fname) {
                        let real_size = platform::get_icloud_real_size(&path).unwrap_or(size);
                        // Strip `.` prefix and `.icloud` suffix for display
                        let original = fname[1..fname.len() - 7].to_string();
                        (original, real_size)
                    } else {
                        (fname.clone(), size)
                    };

                    let extension = Path::new(&display_name).extension().map(|e| e.to_string_lossy().to_lowercase());
                    files.push(FileNode {
                        name: display_name,
                        path: path.to_string_lossy().to_string(),
                        size: final_size,
                        is_dir: false,
                        children: Vec::new(),
                        file_count: 1,
                        extension,
                        dir_count: 0,
                        modified,
                        is_symlink,
                        is_hidden,
                        is_readonly,
                        cleanup_pattern_id: None,
                    });

                    bytes += final_size;
                    count += 1;
                }
            }
            Err(e) => {
                let msg = e.to_string();
                let level = if is_ignorable_error(&msg, dir_path) { "skip" } else { "warn" };
                send_log(&on_log, level, &format!("Cannot read dir: {}", msg), dir_path);
            }
        }

        total_files.fetch_add(count, Ordering::Relaxed);
        total_bytes.fetch_add(bytes, Ordering::Relaxed);
        completed_dirs.fetch_add(1, Ordering::Relaxed);

        if count > 0 {
            dir_sizes_map.insert(dir_path.clone(), (bytes, count));
        }
        if !files.is_empty() {
            file_results.insert(dir_path.clone(), files);
        }
    });

    // 타이머 스레드 종료
    timer_running.store(false, Ordering::Relaxed);
    let _ = timer_thread.join();

    if cancel_token.load(Ordering::Relaxed) {
        send_log(&on_log, "info", "Scan cancelled by user", root);
        return None;
    }

    // 최종 트리 조립 — convert DashMap to HashMap
    let mut file_map: HashMap<String, Vec<FileNode>> = match Arc::try_unwrap(file_results) {
        Ok(map) => map.into_iter().collect(),
        Err(arc) => arc.iter().map(|entry| (entry.key().clone(), entry.value().clone())).collect(),
    };

    let final_tree = scanner_tree::assemble_tree(&skeleton, &mut file_map);

    let tf = total_files.load(Ordering::Relaxed);
    let tb = total_bytes.load(Ordering::Relaxed);
    send_log(&on_log, "info", &format!("Scan complete: {} files, {} bytes", tf, tb), root);

    Some(scanner_tree::prune_tree(final_tree, options.max_children_per_dir))
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

pub(crate) fn send_log(channel: &Channel<ScanLogEntry>, level: &str, message: &str, path: &str) {
    if level == "skip" {
        let count = SKIP_LOG_COUNT.fetch_add(1, Ordering::Relaxed);
        if count % 100 != 0 { return; }
    }
    let _ = channel.send(ScanLogEntry {
        level: level.to_string(), message: message.to_string(), path: path.to_string(),
    });
}
