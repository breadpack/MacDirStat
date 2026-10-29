use std::sync::atomic::AtomicBool;
use std::sync::{Arc, RwLock};

use crate::scanner::write_lock;
use crate::tree::ScanTree;

pub struct ScanState {
    pub cancel_token: Arc<AtomicBool>,
    /// 마지막 스캔 결과. 프론트엔드는 필요한 부분만 조회한다.
    pub tree: Arc<RwLock<Option<ScanTree>>>,
}

impl ScanState {
    pub fn new() -> Self {
        Self {
            cancel_token: Arc::new(AtomicBool::new(false)),
            tree: Arc::new(RwLock::new(None)),
        }
    }

    /// 파일 시스템에서 삭제된 경로를 트리에서도 제거한다.
    pub fn remove_path(&self, path: &str) {
        let mut guard = write_lock(&self.tree);
        if let Some(tree) = guard.as_mut() {
            if let Some(id) = tree.resolve(path) {
                tree.remove(id);
            }
        }
    }
}
