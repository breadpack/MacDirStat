use std::sync::atomic::Ordering;
use std::sync::{Arc, RwLock};

use tauri::ipc::Channel;
use tauri::State;

use crate::models::{ExtensionStat, NodeDto, ScanLogEntry, ScanProgress, TreemapNode};
use crate::scanner::{self, read_lock, write_lock, ScanReporter};
use crate::state::ScanState;
use crate::tree::ScanTree;

struct ChannelReporter {
    progress: Channel<ScanProgress>,
    log: Channel<ScanLogEntry>,
}

impl ScanReporter for ChannelReporter {
    fn progress(&self, progress: ScanProgress) {
        let _ = self.progress.send(progress);
    }

    fn log(&self, level: &str, message: &str, path: &str) {
        let _ = self.log.send(ScanLogEntry {
            level: level.to_string(),
            message: message.to_string(),
            path: path.to_string(),
        });
    }
}

fn build_scan_options(
    exclude_paths: Option<Vec<String>>,
    exclude_names: Option<Vec<String>>,
    follow_symlinks: Option<bool>,
    max_children_per_dir: Option<usize>,
) -> scanner::ScanOptions {
    scanner::ScanOptions {
        exclude_paths: exclude_paths.unwrap_or_else(|| {
            scanner::DEFAULT_SKIP_PATHS.iter().map(|s| s.to_string()).collect()
        }),
        exclude_names: exclude_names.unwrap_or_else(|| {
            scanner::DEFAULT_SKIP_NAMES.iter().map(|s| s.to_string()).collect()
        }),
        follow_symlinks: follow_symlinks.unwrap_or(false),
        max_children_per_dir: match max_children_per_dir.unwrap_or(200) {
            0 => usize::MAX,
            n => n,
        },
        cross_device: false,
    }
}

/// 전체 스캔. 트리는 백엔드 상태에 보관되고 루트 노드 요약만 반환된다.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn scan_directory(
    path: String,
    exclude_paths: Option<Vec<String>>,
    exclude_names: Option<Vec<String>>,
    follow_symlinks: Option<bool>,
    max_children_per_dir: Option<usize>,
    on_progress: Channel<ScanProgress>,
    on_log: Channel<ScanLogEntry>,
    state: State<'_, ScanState>,
) -> Result<Option<NodeDto>, String> {
    state.cancel_token.store(false, Ordering::Relaxed);
    // 이전 결과를 먼저 해제하여 두 트리가 동시에 메모리에 올라가지 않게 한다
    *write_lock(&state.tree) = None;

    let options = build_scan_options(exclude_paths, exclude_names, follow_symlinks, max_children_per_dir);
    let cancel_token = Arc::clone(&state.cancel_token);
    let slot = Arc::clone(&state.tree);
    tauri::async_runtime::spawn_blocking(move || {
        let reporter = ChannelReporter { progress: on_progress, log: on_log };
        if scanner::scan_into(&path, &cancel_token, &reporter, &options, &slot) {
            read_lock(&slot).as_ref().map(ScanTree::root_dto)
        } else {
            *write_lock(&slot) = None;
            None
        }
    })
    .await
    .map_err(|e| e.to_string())
}

/// path 서브트리만 다시 스캔하여 기존 트리에 교체 반영한다. 갱신된 루트 노드 요약을 반환한다.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn refresh_subtree(
    path: String,
    exclude_paths: Option<Vec<String>>,
    exclude_names: Option<Vec<String>>,
    follow_symlinks: Option<bool>,
    max_children_per_dir: Option<usize>,
    on_progress: Channel<ScanProgress>,
    on_log: Channel<ScanLogEntry>,
    state: State<'_, ScanState>,
) -> Result<Option<NodeDto>, String> {
    state.cancel_token.store(false, Ordering::Relaxed);

    let options = build_scan_options(exclude_paths, exclude_names, follow_symlinks, max_children_per_dir);
    let cancel_token = Arc::clone(&state.cancel_token);
    let main = Arc::clone(&state.tree);
    tauri::async_runtime::spawn_blocking(move || {
        let reporter = ChannelReporter { progress: on_progress, log: on_log };
        let local = RwLock::new(None);
        if !scanner::scan_into(&path, &cancel_token, &reporter, &options, &local) {
            return None;
        }
        let sub = local.into_inner().unwrap_or_else(|e| e.into_inner())?;
        let mut guard = write_lock(&main);
        let tree = guard.as_mut()?;
        let target = tree.resolve(&path)?;
        if !tree.node(target).is_dir() {
            return None;
        }
        tree.graft(target, &sub);
        Some(tree.root_dto())
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn cancel_scan(state: State<'_, ScanState>) -> Result<(), String> {
    state.cancel_token.store(true, Ordering::Relaxed);
    Ok(())
}

// 조회 커맨드는 async로 두어 메인(UI) 스레드가 아닌 런타임 스레드에서 실행되게 한다.

#[tauri::command]
pub async fn get_node(path: String, state: State<'_, ScanState>) -> Result<Option<NodeDto>, String> {
    Ok(read_lock(&state.tree).as_ref().and_then(|t| t.node_dto(&path)))
}

/// 여러 디렉토리의 자식 목록을 한 번에 조회 (입력 순서와 동일, 없는 경로는 null)
#[tauri::command]
pub async fn get_children(paths: Vec<String>, state: State<'_, ScanState>) -> Result<Vec<Option<Vec<NodeDto>>>, String> {
    let guard = read_lock(&state.tree);
    Ok(match guard.as_ref() {
        Some(tree) => paths.iter().map(|p| tree.children_dtos(p)).collect(),
        None => vec![None; paths.len()],
    })
}

#[tauri::command]
pub async fn get_treemap(
    path: String,
    min_ratio: f64,
    max_children: usize,
    max_nodes: usize,
    state: State<'_, ScanState>,
) -> Result<Option<TreemapNode>, String> {
    Ok(read_lock(&state.tree)
        .as_ref()
        .and_then(|t| t.treemap(&path, min_ratio, max_children, max_nodes)))
}

#[tauri::command]
pub async fn get_extension_stats(path: String, state: State<'_, ScanState>) -> Result<Vec<ExtensionStat>, String> {
    Ok(read_lock(&state.tree)
        .as_ref()
        .map(|t| t.extension_stats(&path))
        .unwrap_or_default())
}
