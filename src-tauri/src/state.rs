use std::sync::atomic::AtomicBool;
use std::sync::Arc;

pub struct ScanState {
    pub cancel_token: Arc<AtomicBool>,
}

impl ScanState {
    pub fn new() -> Self {
        Self {
            cancel_token: Arc::new(AtomicBool::new(false)),
        }
    }
}
