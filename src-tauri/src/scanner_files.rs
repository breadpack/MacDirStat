use std::cmp::Ordering as CmpOrdering;
use std::collections::BinaryHeap;
use std::path::Path;
use std::time::UNIX_EPOCH;

use dashmap::DashSet;

use crate::platform;
use crate::scanner::{is_ignorable_error, send_log, ScanReporter};
use crate::tree::{DirResult, FileEntry, NodeId, FLAG_HIDDEN, FLAG_READONLY, FLAG_SYMLINK};

/// 크기 기준 min-heap 원소 (가장 작은 파일이 top)
struct BySizeAsc(FileEntry);

impl PartialEq for BySizeAsc {
    fn eq(&self, other: &Self) -> bool {
        self.0.size == other.0.size
    }
}
impl Eq for BySizeAsc {}
impl PartialOrd for BySizeAsc {
    fn partial_cmp(&self, other: &Self) -> Option<CmpOrdering> {
        Some(self.cmp(other))
    }
}
impl Ord for BySizeAsc {
    fn cmp(&self, other: &Self) -> CmpOrdering {
        other.0.size.cmp(&self.0.size)
    }
}

struct FileMeta {
    size: u64,
    modified: u64,
    readonly: bool,
    nlink: u64,
    ino: u64,
}

fn read_meta(entry: &std::fs::DirEntry, reporter: &dyn ScanReporter) -> FileMeta {
    match entry.metadata() {
        Ok(m) => {
            let modified = m
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            #[cfg(unix)]
            {
                use std::os::unix::fs::{MetadataExt, PermissionsExt};
                // Use physical disk usage (blocks * 512) instead of logical size
                let blocks = m.blocks();
                let size = if blocks > 0 { blocks * 512 } else { m.len() };
                FileMeta { size, modified, readonly: m.permissions().mode() & 0o222 == 0, nlink: m.nlink(), ino: m.ino() }
            }
            #[cfg(not(unix))]
            {
                FileMeta { size: m.len(), modified, readonly: m.permissions().readonly(), nlink: 1, ino: 0 }
            }
        }
        Err(e) => {
            let ps = entry.path().to_string_lossy().to_string();
            let msg = e.to_string();
            let level = if is_ignorable_error(&msg, &ps) { "skip" } else { "warn" };
            send_log(reporter, level, &format!("Cannot read metadata: {}", msg), &ps);
            FileMeta { size: 0, modified: 0, readonly: false, nlink: 1, ino: 0 }
        }
    }
}

/// 디렉토리 하나의 파일들을 읽는다. 크기 상위 max_files개만 FileEntry로 만들고 나머지는 합산한다.
/// 상위에 들지 못한 파일은 이름 문자열조차 할당하지 않는다.
pub fn scan_dir_files(
    dir: NodeId,
    dir_path: &Path,
    max_files: usize,
    seen_inodes: &DashSet<u64>,
    reporter: &dyn ScanReporter,
) -> DirResult {
    let mut result = DirResult {
        dir,
        files: Vec::new(),
        others_count: 0,
        others_bytes: 0,
        others_modified: 0,
        bytes: 0,
        count: 0,
        modified: 0,
    };
    let mut heap: BinaryHeap<BySizeAsc> = BinaryHeap::new();

    let entries = match std::fs::read_dir(dir_path) {
        Ok(e) => e,
        Err(e) => {
            let ps = dir_path.to_string_lossy();
            let msg = e.to_string();
            let level = if is_ignorable_error(&msg, &ps) { "skip" } else { "warn" };
            send_log(reporter, level, &format!("Cannot read dir: {}", msg), &ps);
            return result;
        }
    };

    for entry in entries.flatten() {
        let ft = match entry.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };
        if !ft.is_file() {
            continue;
        }

        let meta = read_meta(&entry, reporter);
        // Skip hardlinked files already counted
        if meta.nlink > 1 && meta.ino != 0 && !seen_inodes.insert(meta.ino) {
            continue;
        }

        let fname = entry.file_name();
        let fname = fname.to_string_lossy();
        let mut size = meta.size;
        let icloud = platform::is_icloud_placeholder(&fname);
        if icloud {
            size = platform::get_icloud_real_size(&entry.path()).unwrap_or(size);
        }

        result.bytes += size;
        result.count += 1;
        result.modified = result.modified.max(meta.modified);

        let fits = heap.len() < max_files || heap.peek().is_some_and(|min| min.0.size < size);
        if !fits {
            result.others_count += 1;
            result.others_bytes += size;
            result.others_modified = result.others_modified.max(meta.modified);
            continue;
        }

        // iCloud placeholder: strip `.` prefix and `.icloud` suffix for display
        let display_name = if icloud { fname[1..fname.len() - 7].to_string() } else { fname.to_string() };
        let ext = Path::new(&display_name).extension().map(|e| e.to_string_lossy().to_lowercase());
        let mut flags = 0;
        if fname.starts_with('.') {
            flags |= FLAG_HIDDEN;
        }
        if ft.is_symlink() {
            flags |= FLAG_SYMLINK;
        }
        if meta.readonly {
            flags |= FLAG_READONLY;
        }
        heap.push(BySizeAsc(FileEntry { name: display_name, size, modified: meta.modified, ext, flags }));

        if heap.len() > max_files {
            if let Some(BySizeAsc(evicted)) = heap.pop() {
                result.others_count += 1;
                result.others_bytes += evicted.size;
                result.others_modified = result.others_modified.max(evicted.modified);
            }
        }
    }

    result.files = heap.into_iter().map(|e| e.0).collect();
    result.files.sort_unstable_by(|a, b| b.size.cmp(&a.size));
    result
}
