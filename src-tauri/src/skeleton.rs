use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::cleanup_patterns::{self, CleanupPattern};
use crate::scanner::{is_ignorable_error, make_progress, send_log, should_skip_dynamic, ScanOptions, ScanReporter};
use crate::models::ScanProgress;
use crate::tree::{NodeId, ScanTree, FLAG_DIR, FLAG_HIDDEN};

/// Phase 1: 디렉토리 구조만 스캔하여 arena 트리를 만든다.
/// 한 디렉토리의 하위 디렉토리들은 연속으로 push되므로 자식 구간이 연속성을 가진다.
pub fn build_skeleton(
    root: &Path,
    cancel_token: &AtomicBool,
    reporter: &dyn ScanReporter,
    options: &ScanOptions,
    root_dev: Option<u64>,
    scan_pattern_map: &HashMap<String, Vec<usize>>,
    all_patterns: &[CleanupPattern],
) -> Option<ScanTree> {
    // Track visited inodes to detect circular symlinks
    let mut visited_inodes: HashSet<u64> = HashSet::new();

    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if let Ok(m) = std::fs::metadata(root) {
            visited_inodes.insert(m.ino());
        }
    }

    let root_str = root.to_string_lossy().to_string();
    let root_name = root
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| root_str.clone());
    let root_flags = if root_name.starts_with('.') { FLAG_HIDDEN } else { 0 };
    let cleanup_ids = all_patterns.iter().map(|p| p.id).collect();
    let mut tree = ScanTree::new(&root_str, &root_name, root_flags, cleanup_ids);

    let mut dir_count: u32 = 0;
    let mut stack: Vec<(PathBuf, NodeId)> = vec![(root.to_path_buf(), 0)];
    let mut child_frames: Vec<(PathBuf, NodeId)> = Vec::new();

    while let Some((path, node_id)) = stack.pop() {
        if cancel_token.load(Ordering::Relaxed) {
            return None;
        }

        let entries = match std::fs::read_dir(&path) {
            Ok(e) => e,
            Err(e) => {
                let path_str = path.to_string_lossy();
                let level = if is_ignorable_error(&e.to_string(), &path_str) { "skip" } else { "warn" };
                send_log(reporter, level, &format!("Cannot read: {}", e), &path_str);
                continue;
            }
        };

        let dirs_start = tree.len() as NodeId;
        child_frames.clear();

        for entry in entries.flatten() {
            let ft = match entry.file_type() {
                Ok(ft) => ft,
                Err(_) => continue,
            };
            if !ft.is_dir() {
                continue;
            }
            let child_path = entry.path();
            if should_skip_dynamic(&child_path, &options.exclude_paths, &options.exclude_names) {
                continue;
            }
            if ft.is_symlink() && !options.follow_symlinks {
                continue;
            }

            #[cfg(unix)]
            {
                use std::os::unix::fs::MetadataExt;
                if let Ok(m) = std::fs::metadata(&child_path) {
                    // Mount point boundary detection
                    if !options.cross_device {
                        if let Some(rd) = root_dev {
                            if m.dev() != rd {
                                continue;
                            }
                        }
                    }
                    // Circular symlink detection via inode tracking
                    if !visited_inodes.insert(m.ino()) {
                        send_log(reporter, "skip", "Skipping circular directory reference", &child_path.to_string_lossy());
                        continue;
                    }
                }
            }

            dir_count += 1;
            if dir_count % 100 == 0 {
                reporter.progress(ScanProgress {
                    current_path: child_path.to_string_lossy().to_string(),
                    total_dirs: dir_count,
                    ..make_progress("structure")
                });
            }

            let child_name = entry.file_name().to_string_lossy().to_string();
            let flags = FLAG_DIR | if child_name.starts_with('.') { FLAG_HIDDEN } else { 0 };
            let cleanup = scan_pattern_map.get(&child_name).and_then(|indices| {
                indices
                    .iter()
                    .find(|&&i| cleanup_patterns::matches_path_pattern(&all_patterns[i], &child_path))
                    .map(|&i| i as u16 + 1)
            });

            let child_id = tree.push_node(&child_name, node_id, flags);
            if let Some(c) = cleanup {
                tree.node_mut(child_id).cleanup = c;
            }
            child_frames.push((child_path, child_id));
        }

        let n = tree.node_mut(node_id);
        n.dirs_start = dirs_start;
        n.dirs_len = child_frames.len() as u32;

        // Push in reverse order so we process in original order
        stack.extend(child_frames.drain(..).rev());
    }

    tree.compute_dir_counts();
    Some(tree)
}
