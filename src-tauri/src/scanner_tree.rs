use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tauri::ipc::Channel;

use crate::cleanup_patterns::{self, CleanupPattern};
use crate::models::{FileNode, ScanLogEntry, ScanProgress};
use crate::scanner::{is_ignorable_error, make_progress, send_log, should_skip_dynamic, ScanOptions};

/// skeleton 트리에서 모든 디렉토리 경로를 수집
pub fn collect_all_dir_paths(node: &FileNode, paths: &mut Vec<String>) {
    paths.push(node.path.clone());
    for child in &node.children {
        collect_all_dir_paths(child, paths);
    }
}

/// skeleton 트리에 스캔된 파일을 결합하여 최종 트리 생성 (iterative, stack-safe)
pub fn assemble_tree(skeleton: &FileNode, file_map: &mut HashMap<String, Vec<FileNode>>) -> FileNode {
    struct NodeInfo<'a> {
        skeleton: &'a FileNode,
        child_indices: Vec<usize>,
    }

    // Phase 1: Flatten the skeleton tree in pre-order, recording parent-child relationships
    let mut nodes: Vec<NodeInfo> = Vec::new();
    let mut stack: Vec<(&FileNode, Option<usize>)> = vec![(skeleton, None)];

    while let Some((skel, parent_idx)) = stack.pop() {
        let current_idx = nodes.len();
        if let Some(pi) = parent_idx {
            nodes[pi].child_indices.push(current_idx);
        }
        nodes.push(NodeInfo {
            skeleton: skel,
            child_indices: Vec::new(),
        });
        // Push children in reverse so they appear in order
        for child in skel.children.iter().rev() {
            stack.push((child, Some(current_idx)));
        }
    }

    // Phase 2: Build FileNodes bottom-up (reverse order ensures children built before parents)
    let mut results: Vec<Option<FileNode>> = vec![None; nodes.len()];

    for i in (0..nodes.len()).rev() {
        let info = &nodes[i];
        let skel = info.skeleton;

        let mut children: Vec<FileNode> = Vec::with_capacity(info.child_indices.len());
        for &ci in &info.child_indices {
            if let Some(child) = results[ci].take() {
                children.push(child);
            }
        }

        // Add file children
        if let Some(files) = file_map.remove(&skel.path) {
            children.extend(files);
        }

        children.sort_by(|a, b| b.size.cmp(&a.size));
        let size: u64 = children.iter().map(|c| c.size).sum();
        let file_count: u64 = children.iter().map(|c| c.file_count).sum();
        let dir_count: u64 = children.iter()
            .filter(|c| c.is_dir)
            .map(|c| 1 + c.dir_count)
            .sum();
        // Bubble up the latest modified time from children
        let modified: Option<u64> = children.iter()
            .filter_map(|c| c.modified)
            .max();
        let is_hidden = skel.name.starts_with('.');

        results[i] = Some(FileNode {
            name: skel.name.clone(),
            path: skel.path.clone(),
            size,
            is_dir: true,
            children,
            file_count,
            dir_count,
            modified,
            is_symlink: false,
            is_hidden,
            is_readonly: false,
            extension: None,
            cleanup_pattern_id: skel.cleanup_pattern_id.clone(),
        });
    }

    results[0].take().unwrap_or_else(|| skeleton.clone())
}

pub fn build_skeleton(
    root: &Path, cancel_token: &Arc<AtomicBool>,
    on_progress: &Channel<ScanProgress>, on_log: &Channel<ScanLogEntry>,
    dir_count: &mut u32, options: &ScanOptions, root_dev: Option<u64>,
    scan_pattern_map: &HashMap<String, Vec<usize>>, all_patterns: &[CleanupPattern],
) -> Option<FileNode> {
    // Fix 3: Track visited inodes to detect circular symlinks
    let mut visited_inodes: HashSet<u64> = HashSet::new();

    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if let Ok(m) = std::fs::metadata(root) {
            visited_inodes.insert(m.ino());
        }
    }

    // Fix 4: Iterative traversal using an explicit stack
    struct StackFrame {
        path: std::path::PathBuf,
        node_index: usize,
    }

    struct FlatNode {
        name: String,
        path: String,
        is_hidden: bool,
        children_indices: Vec<usize>,
        cleanup_pattern_id: Option<String>,
    }

    let root_str = root.to_string_lossy().to_string();
    let root_name = root.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| root_str.clone());

    let mut nodes: Vec<FlatNode> = vec![FlatNode {
        name: root_name,
        path: root_str,
        is_hidden: root.file_name().map(|n| n.to_string_lossy().starts_with('.')).unwrap_or(false),
        children_indices: Vec::new(),
        cleanup_pattern_id: None,
    }];

    let mut stack: Vec<StackFrame> = vec![StackFrame {
        path: root.to_path_buf(),
        node_index: 0,
    }];

    while let Some(frame) = stack.pop() {
        if cancel_token.load(Ordering::Relaxed) { return None; }

        let path_str = frame.path.to_string_lossy().to_string();
        let entries = match std::fs::read_dir(&frame.path) {
            Ok(e) => e,
            Err(e) => {
                let level = if is_ignorable_error(&e.to_string(), &path_str) { "skip" } else { "warn" };
                send_log(on_log, level, &format!("Cannot read: {}", e), &path_str);
                continue;
            }
        };

        let mut child_frames: Vec<StackFrame> = Vec::new();

        for entry in entries.flatten() {
            if cancel_token.load(Ordering::Relaxed) { return None; }
            let ft = match entry.file_type() { Ok(ft) => ft, Err(_) => continue };
            let child_path = entry.path();

            if ft.is_dir() {
                if should_skip_dynamic(&child_path, &options.exclude_paths, &options.exclude_names) { continue; }
                if ft.is_symlink() && !options.follow_symlinks { continue; }

                // Fix 3: Circular symlink detection via inode tracking
                #[cfg(unix)]
                {
                    use std::os::unix::fs::MetadataExt;
                    if let Ok(m) = std::fs::metadata(&child_path) {
                        // Fix 5: Mount point boundary detection
                        if !options.cross_device {
                            if let Some(rd) = root_dev {
                                if m.dev() != rd {
                                    continue;
                                }
                            }
                        }

                        let ino = m.ino();
                        if !visited_inodes.insert(ino) {
                            let child_str = child_path.to_string_lossy().to_string();
                            send_log(on_log, "skip", "Skipping circular directory reference", &child_str);
                            continue;
                        }
                    }
                }

                let child_str = child_path.to_string_lossy().to_string();
                *dir_count += 1;
                if *dir_count % 100 == 0 {
                    let _ = on_progress.send(ScanProgress {
                        current_path: child_str.clone(),
                        total_dirs: *dir_count,
                        ..make_progress("structure")
                    });
                }

                let child_name = child_path.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| child_str.clone());
                let is_hidden = child_name.starts_with('.');

                let cleanup_id = scan_pattern_map.get(&child_name)
                    .and_then(|indices| {
                        indices.iter().find_map(|&i| {
                            if cleanup_patterns::matches_path_pattern(&all_patterns[i], &child_path) {
                                Some(all_patterns[i].id.to_string())
                            } else {
                                None
                            }
                        })
                    });

                let child_index = nodes.len();
                nodes.push(FlatNode {
                    name: child_name,
                    path: child_str,
                    is_hidden,
                    children_indices: Vec::new(),
                    cleanup_pattern_id: cleanup_id,
                });
                nodes[frame.node_index].children_indices.push(child_index);

                child_frames.push(StackFrame {
                    path: child_path,
                    node_index: child_index,
                });
            }
        }

        // Push in reverse order so we process in original order
        for cf in child_frames.into_iter().rev() {
            stack.push(cf);
        }
    }

    fn build_tree(nodes: &mut Vec<FlatNode>, index: usize) -> FileNode {
        let children_indices = std::mem::take(&mut nodes[index].children_indices);
        let children: Vec<FileNode> = children_indices.into_iter()
            .map(|ci| build_tree(nodes, ci))
            .collect();

        FileNode {
            name: nodes[index].name.clone(),
            path: nodes[index].path.clone(),
            size: 0,
            is_dir: true,
            children,
            file_count: 0,
            extension: None,
            dir_count: 0,
            modified: None,
            is_symlink: false,
            is_hidden: nodes[index].is_hidden,
            is_readonly: false,
            cleanup_pattern_id: nodes[index].cleanup_pattern_id.clone(),
        }
    }

    Some(build_tree(&mut nodes, 0))
}

/// Prune tree to limit children per directory (iterative, stack-safe)
pub fn prune_tree(root: FileNode, max_children: usize) -> FileNode {
    struct NodeInfo {
        node: FileNode,
        child_indices: Vec<usize>,
    }

    // Phase 1: Flatten the tree in pre-order, extracting children
    let mut nodes: Vec<NodeInfo> = Vec::new();
    // Stack holds (FileNode, parent_idx)
    let mut stack: Vec<(FileNode, Option<usize>)> = vec![(root, None)];

    while let Some((mut node, parent_idx)) = stack.pop() {
        let current_idx = nodes.len();
        if let Some(pi) = parent_idx {
            nodes[pi].child_indices.push(current_idx);
        }

        // Take children out of node before storing
        let children = std::mem::take(&mut node.children);
        nodes.push(NodeInfo {
            node,
            child_indices: Vec::new(),
        });

        // Push children in reverse so they come out in order
        for child in children.into_iter().rev() {
            stack.push((child, Some(current_idx)));
        }
    }

    // Phase 2: Process bottom-up, applying pruning logic
    let mut results: Vec<Option<FileNode>> = vec![None; nodes.len()];

    for i in (0..nodes.len()).rev() {
        let info = &nodes[i];
        let node = &info.node;

        if !node.is_dir || info.child_indices.is_empty() {
            // Leaf or file node — return as-is (with empty children already taken)
            results[i] = Some(FileNode {
                children: Vec::new(),
                ..node.clone()
            });
            continue;
        }

        // Collect built children
        let built_children: Vec<FileNode> = info.child_indices.iter()
            .filter_map(|&ci| results[ci].take())
            .collect();

        // Partition into dirs and files
        let (dirs, files): (Vec<FileNode>, Vec<FileNode>) = built_children.into_iter()
            .partition(|c| c.is_dir);

        let mut children: Vec<FileNode> = dirs;

        if files.len() <= max_children {
            children.extend(files);
        } else {
            let mut files = files;
            let rest = files.split_off(max_children);
            let others_size: u64 = rest.iter().map(|c| c.size).sum();
            let others_fc: u64 = rest.iter().map(|c| c.file_count).sum();
            children.extend(files);
            children.push(FileNode {
                name: format!("({} other files)", rest.len()),
                path: format!("{}/__others__", node.path),
                size: others_size, is_dir: false, children: Vec::new(),
                file_count: others_fc, extension: None,
                dir_count: 0, modified: None, is_symlink: false, is_hidden: false, is_readonly: false,
                cleanup_pattern_id: None,
            });
        }

        children.sort_by(|a, b| b.size.cmp(&a.size));

        results[i] = Some(FileNode {
            children,
            ..node.clone()
        });
    }

    results[0].take().unwrap_or_else(|| FileNode {
        name: String::new(),
        path: String::new(),
        size: 0,
        is_dir: true,
        children: Vec::new(),
        file_count: 0,
        extension: None,
        dir_count: 0,
        modified: None,
        is_symlink: false,
        is_hidden: false,
        is_readonly: false,
        cleanup_pattern_id: None,
    })
}
