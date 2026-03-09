use std::ffi::CString;
use std::path::Path;
use sysinfo::Disks;

use crate::models::VolumeInfo;

/// Try to read the real file size for an iCloud placeholder file.
///
/// iCloud placeholder files have the pattern `.filename.icloud`.
/// The real size may be stored in the extended attribute "com.apple.ubiquity.itemSize".
/// If that fails, we try to stat the original filename (without `.` prefix and `.icloud` suffix).
pub fn get_icloud_real_size(path: &Path) -> Option<u64> {
    let c_path = CString::new(path.to_str()?).ok()?;

    // Try reading the xattr "com.apple.ubiquity.itemSize"
    let attr_name = CString::new("com.apple.ubiquity.itemSize").unwrap();
    let mut buf = [0u8; 8];
    let len = unsafe {
        libc::getxattr(
            c_path.as_ptr(),
            attr_name.as_ptr(),
            buf.as_mut_ptr() as *mut libc::c_void,
            buf.len(),
            0,
            0,
        )
    };
    if len == 8 {
        // The xattr stores a big-endian u64
        return Some(u64::from_be_bytes(buf));
    }

    // Fallback: try to stat the original file (without .prefix and .icloud suffix)
    let file_name = path.file_name()?.to_str()?;
    if let Some(original_name) = file_name.strip_prefix('.').and_then(|n| n.strip_suffix(".icloud")) {
        let original_path = path.parent()?.join(original_name);
        if let Ok(meta) = std::fs::metadata(&original_path) {
            return Some(meta.len());
        }
    }

    None
}

/// Check if a filename is an iCloud placeholder (`.XXX.icloud` pattern).
pub fn is_icloud_placeholder(name: &str) -> bool {
    name.starts_with('.') && name.ends_with(".icloud") && name.len() > 8
}

/// Full Disk Access 여부를 확인한다.
/// metadata()는 성공하지만 read_dir()이 실패하는 경로로 테스트해야 정확하다.
pub fn check_full_disk_access() -> bool {
    let home = match std::env::var("HOME") {
        Ok(h) if !h.is_empty() => h,
        _ => {
            // HOME을 가져올 수 없으면 안전하게 false 반환
            return false;
        }
    };

    // FDA가 없으면 이 디렉토리들의 내용을 읽을 수 없다
    let test_paths = [
        format!("{}/Library/Safari", home),
        format!("{}/Library/Mail", home),
        format!("{}/Library/Containers/com.apple.Safari", home),
    ];

    for path in &test_paths {
        let p = Path::new(path);
        // 디렉토리가 존재하는지 먼저 확인 (앱이 설치 안 된 경우 skip)
        if !p.is_dir() {
            continue;
        }
        // read_dir 시도 - FDA 없으면 Permission denied
        match std::fs::read_dir(p) {
            Ok(_) => return true,
            Err(_) => return false,
        }
    }

    // 테스트 경로가 하나도 없으면 (Safari, Mail 미설치), TCC.db로 폴백
    let tcc = Path::new("/Library/Application Support/com.apple.TCC/TCC.db");
    if tcc.exists() {
        return std::fs::read(tcc).is_ok();
    }

    // 판단 불가 시 false로 처리 (사용자에게 FDA 설정을 안내)
    false
}

/// macOS에서 SIP 등으로 항상 접근 불가한 경로인지 확인한다.
/// 이런 경로의 에러는 warn이 아니라 skip으로 처리해야 한다.
pub fn is_system_protected(path: &str) -> bool {
    const PROTECTED_PREFIXES: &[&str] = &[
        "/System/",
        "/usr/",
        "/bin/",
        "/sbin/",
        "/private/var/db/",
        "/private/var/folders/",
        "/Library/Application Support/com.apple.TCC",
    ];

    for prefix in PROTECTED_PREFIXES {
        if path.starts_with(prefix) {
            return true;
        }
    }
    false
}

pub fn open_full_disk_access_settings() -> Result<(), String> {
    std::process::Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles")
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn get_volumes() -> Vec<VolumeInfo> {
    let disks = Disks::new_with_refreshed_list();
    disks
        .iter()
        .map(|d| VolumeInfo {
            name: d.name().to_string_lossy().to_string(),
            mount_point: d.mount_point().to_string_lossy().to_string(),
            total_bytes: d.total_space(),
            available_bytes: d.available_space(),
        })
        .collect()
}

pub fn open_in_finder(path: &str) -> Result<(), String> {
    std::process::Command::new("open")
        .arg("-R")
        .arg(path)
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn show_get_info(path: &str) -> Result<(), String> {
    std::process::Command::new("osascript")
        .arg("-e")
        .arg(&format!(
            r#"tell application "Finder" to open information window of (POSIX file "{}" as alias)"#,
            path
        ))
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn open_in_terminal(path: &str) -> Result<(), String> {
    let dir = if Path::new(path).is_dir() {
        path.to_string()
    } else {
        Path::new(path)
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "/".to_string())
    };
    std::process::Command::new("open")
        .arg("-a")
        .arg("Terminal")
        .arg(&dir)
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn open_file(path: &str) -> Result<(), String> {
    std::process::Command::new("open")
        .arg(path)
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}
