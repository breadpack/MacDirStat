use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::models::FileNode;

pub struct ScanState {
    pub cancel_token: Arc<AtomicBool>,
    pub tree: Mutex<Option<FileNode>>,
}

impl ScanState {
    pub fn new() -> Self {
        Self {
            cancel_token: Arc::new(AtomicBool::new(false)),
            tree: Mutex::new(None),
        }
    }
}
