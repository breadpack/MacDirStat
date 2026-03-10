# Cleanup Recommendations Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add automatic detection, recommendation, and execution of macOS disk space cleanup opportunities, integrated with scan results and as a standalone system scan.

**Architecture:** Rust backend with `cleanup_patterns/` module containing categorized pattern definitions. Scanner integration via dir_name HashMap lookup during skeleton build. Standalone scan via new Tauri commands checking known paths and running system commands. Frontend modal for recommendations + TreeView/Treemap visual indicators.

**Tech Stack:** Rust (Tauri v2, serde, trash crate), Svelte 5 (runes, writable stores), TypeScript, HTML5 Canvas

---

### Task 1: Rust Pattern Model & Types

**Files:**
- Create: `src-tauri/src/cleanup_patterns/mod.rs`
- Modify: `src-tauri/src/models.rs:23-41` (FileNode struct)
- Modify: `src-tauri/src/lib.rs:1` (module declaration)

**Step 1: Add cleanup_patterns module declaration to lib.rs**

In `src-tauri/src/lib.rs`, add the module declaration. Currently the file starts with `pub fn run()`. We need to add `mod cleanup_patterns;` at the top of the crate. Check `src-tauri/src/main.rs` for where modules are declared:

```rust
// In main.rs or lib.rs where other mods are declared, add:
mod cleanup_patterns;
```

**Step 2: Create the pattern model types**

Create `src-tauri/src/cleanup_patterns/mod.rs`:

```rust
pub mod dev_tools;
pub mod package_managers;
pub mod containers;
pub mod browsers;
pub mod ides;
pub mod system;
pub mod cloud_storage;
pub mod app_data;
pub mod media;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CleanupCategory {
    DevTools,
    PackageManager,
    Container,
    Browser,
    IDE,
    System,
    CloudStorage,
    AppData,
    Media,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RiskLevel {
    Safe,
    Caution,
    Warning,
}

#[derive(Debug, Clone)]
pub enum DetectionMethod {
    PathPattern {
        dir_name: Option<&'static str>,
        path_contains: Option<&'static str>,
        parent_marker: Option<&'static str>,
    },
    KnownPath {
        path: &'static str,
        expandable: bool,
    },
    Command {
        check_cmd: &'static str,
        parse_hint: &'static str,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CleanupMethod {
    Delete { use_trash: bool },
    ShellCommand {
        command: String,
        run_in_terminal: bool,
        refresh_after: bool,
    },
    OpenInFinder,
}

#[derive(Debug, Clone)]
pub struct CleanupPattern {
    pub id: &'static str,
    pub category: CleanupCategory,
    pub name: &'static str,
    pub description: &'static str,
    pub risk_level: RiskLevel,
    pub detection: DetectionMethod,
    pub cleanup: CleanupMethod,
}

/// Serializable pattern info for frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupPatternInfo {
    pub id: String,
    pub category: CleanupCategory,
    pub name: String,
    pub description: String,
    pub risk_level: RiskLevel,
    pub cleanup_method: CleanupMethod,
}

/// Result of standalone cleanup scan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupRecommendation {
    pub pattern_id: String,
    pub pattern_name: String,
    pub category: CleanupCategory,
    pub risk_level: RiskLevel,
    pub description: String,
    pub paths: Vec<String>,
    pub total_size: u64,
    pub cleanup_method: CleanupMethod,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupScanProgress {
    pub current_pattern: String,
    pub checked: usize,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupResult {
    pub success: bool,
    pub freed_bytes: u64,
    pub message: String,
}

/// Collect all patterns from all category modules
pub fn all_patterns() -> Vec<CleanupPattern> {
    let mut patterns = Vec::new();
    patterns.extend(dev_tools::patterns());
    patterns.extend(package_managers::patterns());
    patterns.extend(containers::patterns());
    patterns.extend(browsers::patterns());
    patterns.extend(ides::patterns());
    patterns.extend(system::patterns());
    patterns.extend(cloud_storage::patterns());
    patterns.extend(app_data::patterns());
    patterns.extend(media::patterns());
    patterns
}

/// Build a HashMap<dir_name, Vec<&CleanupPattern>> for scan-time matching
pub fn build_scan_pattern_map(patterns: &[CleanupPattern]) -> HashMap<String, Vec<usize>> {
    let mut map: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, pattern) in patterns.iter().enumerate() {
        if let DetectionMethod::PathPattern { dir_name: Some(name), .. } = &pattern.detection {
            map.entry(name.to_string()).or_default().push(i);
        }
    }
    map
}

/// Check if a path matches a PathPattern-type pattern
pub fn matches_path_pattern(pattern: &CleanupPattern, path: &Path) -> bool {
    match &pattern.detection {
        DetectionMethod::PathPattern { dir_name, path_contains, parent_marker } => {
            if let Some(expected_name) = dir_name {
                let actual_name = path.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                if actual_name != *expected_name {
                    return false;
                }
            }
            if let Some(substr) = path_contains {
                if !path.to_string_lossy().contains(substr) {
                    return false;
                }
            }
            if let Some(marker) = parent_marker {
                if let Some(parent) = path.parent() {
                    if !parent.join(marker).exists() {
                        return false;
                    }
                } else {
                    return false;
                }
            }
            true
        }
        _ => false,
    }
}

/// Expand ~ in path to home directory
pub fn expand_home(path: &str) -> PathBuf {
    if path.starts_with("~/") {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        PathBuf::from(home).join(&path[2..])
    } else {
        PathBuf::from(path)
    }
}

/// Calculate directory size (non-recursive for top-level, recursive for full)
pub fn dir_size(path: &Path) -> u64 {
    if path.is_file() {
        return path.metadata().map(|m| m.len()).unwrap_or(0);
    }
    walkdir::WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.metadata().map(|m| m.len()).unwrap_or(0))
        .sum()
}

impl From<&CleanupPattern> for CleanupPatternInfo {
    fn from(p: &CleanupPattern) -> Self {
        CleanupPatternInfo {
            id: p.id.to_string(),
            category: p.category.clone(),
            name: p.name.to_string(),
            description: p.description.to_string(),
            risk_level: p.risk_level.clone(),
            cleanup_method: p.cleanup.clone(),
        }
    }
}
```

**Step 3: Add `cleanup_pattern_id` field to FileNode**

In `src-tauri/src/models.rs`, add to the FileNode struct after `is_readonly`:

```rust
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cleanup_pattern_id: Option<String>,
```

**Step 4: Add `walkdir` dependency**

```bash
cd src-tauri && cargo add walkdir
```

**Step 5: Update all FileNode construction sites**

Search for all places that construct `FileNode { ... }` in the Rust codebase and add `cleanup_pattern_id: None`. Key locations:
- `scanner_tree.rs:77-90` (assemble_tree)
- `scanner_tree.rs:232-245` (build_tree)
- `scanner_tree.rs:291-294` (prune_tree leaf)
- `scanner_tree.rs:317-323` (prune_tree "others" node)
- `scanner_tree.rs:328-331` (prune_tree dir)
- `scanner_tree.rs:334-347` (prune_tree fallback)
- `scanner.rs` (any FileNode constructions for files)
- `commands.rs` (if any)

**Step 6: Verify compilation**

```bash
cd src-tauri && cargo check
```

**Step 7: Commit**

```bash
git add src-tauri/src/cleanup_patterns/ src-tauri/src/models.rs src-tauri/src/lib.rs src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -m "feat: add cleanup pattern model types and FileNode.cleanup_pattern_id"
```

---

### Task 2: Category Pattern Modules (9 files)

**Files:**
- Create: `src-tauri/src/cleanup_patterns/dev_tools.rs`
- Create: `src-tauri/src/cleanup_patterns/package_managers.rs`
- Create: `src-tauri/src/cleanup_patterns/containers.rs`
- Create: `src-tauri/src/cleanup_patterns/browsers.rs`
- Create: `src-tauri/src/cleanup_patterns/ides.rs`
- Create: `src-tauri/src/cleanup_patterns/system.rs`
- Create: `src-tauri/src/cleanup_patterns/cloud_storage.rs`
- Create: `src-tauri/src/cleanup_patterns/app_data.rs`
- Create: `src-tauri/src/cleanup_patterns/media.rs`

**Step 1: Create dev_tools.rs**

```rust
use super::*;

pub fn patterns() -> Vec<CleanupPattern> {
    vec![
        CleanupPattern {
            id: "xcode-derived-data",
            category: CleanupCategory::DevTools,
            name: "Xcode DerivedData",
            description: "Xcode build cache. Safe to delete, regenerates on next build.",
            risk_level: RiskLevel::Safe,
            detection: DetectionMethod::KnownPath {
                path: "~/Library/Developer/Xcode/DerivedData",
                expandable: true,
            },
            cleanup: CleanupMethod::Delete { use_trash: true },
        },
        CleanupPattern {
            id: "xcode-archives",
            category: CleanupCategory::DevTools,
            name: "Xcode Archives",
            description: "App Store submission archives. Only delete if you no longer need to submit those builds.",
            risk_level: RiskLevel::Caution,
            detection: DetectionMethod::KnownPath {
                path: "~/Library/Developer/Xcode/Archives",
                expandable: true,
            },
            cleanup: CleanupMethod::OpenInFinder,
        },
        CleanupPattern {
            id: "xcode-device-support",
            category: CleanupCategory::DevTools,
            name: "Xcode Device Support",
            description: "Debug symbols for iOS devices. Delete old versions you no longer debug on.",
            risk_level: RiskLevel::Safe,
            detection: DetectionMethod::KnownPath {
                path: "~/Library/Developer/Xcode/iOS DeviceSupport",
                expandable: true,
            },
            cleanup: CleanupMethod::Delete { use_trash: true },
        },
        CleanupPattern {
            id: "xcode-simulators",
            category: CleanupCategory::DevTools,
            name: "iOS Simulators (unavailable)",
            description: "Simulators for uninstalled runtimes. Safe to remove.",
            risk_level: RiskLevel::Safe,
            detection: DetectionMethod::Command {
                check_cmd: "xcrun simctl list devices unavailable 2>/dev/null | grep -c 'unavailable' || echo 0",
                parse_hint: "count_lines",
            },
            cleanup: CleanupMethod::ShellCommand {
                command: "xcrun simctl delete unavailable".to_string(),
                run_in_terminal: true,
                refresh_after: false,
            },
        },
        CleanupPattern {
            id: "xcode-spm-cache",
            category: CleanupCategory::DevTools,
            name: "Swift Package Manager Cache",
            description: "SPM resolved packages cache. Re-downloads on next resolve.",
            risk_level: RiskLevel::Safe,
            detection: DetectionMethod::KnownPath {
                path: "~/Library/Caches/org.swift.swiftpm",
                expandable: true,
            },
            cleanup: CleanupMethod::Delete { use_trash: true },
        },
        CleanupPattern {
            id: "cargo-target",
            category: CleanupCategory::DevTools,
            name: "Rust build artifacts (target/)",
            description: "Cargo build output. Safe to delete, rebuilds on next compile.",
            risk_level: RiskLevel::Safe,
            detection: DetectionMethod::PathPattern {
                dir_name: Some("target"),
                path_contains: None,
                parent_marker: Some("Cargo.toml"),
            },
            cleanup: CleanupMethod::Delete { use_trash: true },
        },
        CleanupPattern {
            id: "gradle-cache",
            category: CleanupCategory::DevTools,
            name: "Gradle Cache",
            description: "Gradle build cache and downloaded dependencies. Re-downloads on next build.",
            risk_level: RiskLevel::Safe,
            detection: DetectionMethod::KnownPath {
                path: "~/.gradle/caches",
                expandable: true,
            },
            cleanup: CleanupMethod::Delete { use_trash: true },
        },
        CleanupPattern {
            id: "maven-repo",
            category: CleanupCategory::DevTools,
            name: "Maven Local Repository",
            description: "Locally cached Maven artifacts. Re-downloads on next build.",
            risk_level: RiskLevel::Safe,
            detection: DetectionMethod::KnownPath {
                path: "~/.m2/repository",
                expandable: true,
            },
            cleanup: CleanupMethod::Delete { use_trash: true },
        },
        CleanupPattern {
            id: "android-sdk",
            category: CleanupCategory::DevTools,
            name: "Android SDK & Emulators",
            description: "Android SDK, system images, and emulators. Review before deleting.",
            risk_level: RiskLevel::Caution,
            detection: DetectionMethod::KnownPath {
                path: "~/Library/Android/sdk",
                expandable: true,
            },
            cleanup: CleanupMethod::OpenInFinder,
        },
    ]
}
```

**Step 2: Create package_managers.rs**

```rust
use super::*;

pub fn patterns() -> Vec<CleanupPattern> {
    vec![
        CleanupPattern {
            id: "node-modules",
            category: CleanupCategory::PackageManager,
            name: "node_modules",
            description: "Node.js dependencies. Restore with npm/pnpm/yarn install.",
            risk_level: RiskLevel::Safe,
            detection: DetectionMethod::PathPattern {
                dir_name: Some("node_modules"),
                path_contains: None,
                parent_marker: Some("package.json"),
            },
            cleanup: CleanupMethod::Delete { use_trash: false },
        },
        CleanupPattern {
            id: "npm-cache",
            category: CleanupCategory::PackageManager,
            name: "npm Cache",
            description: "npm download cache. Packages re-download on demand.",
            risk_level: RiskLevel::Safe,
            detection: DetectionMethod::KnownPath { path: "~/.npm", expandable: true },
            cleanup: CleanupMethod::ShellCommand {
                command: "npm cache clean --force".to_string(),
                run_in_terminal: false,
                refresh_after: false,
            },
        },
        CleanupPattern {
            id: "pnpm-store",
            category: CleanupCategory::PackageManager,
            name: "pnpm Store",
            description: "pnpm content-addressable store. Prunes unreferenced packages.",
            risk_level: RiskLevel::Safe,
            detection: DetectionMethod::KnownPath { path: "~/Library/pnpm/store", expandable: true },
            cleanup: CleanupMethod::ShellCommand {
                command: "pnpm store prune".to_string(),
                run_in_terminal: false,
                refresh_after: false,
            },
        },
        CleanupPattern {
            id: "yarn-cache",
            category: CleanupCategory::PackageManager,
            name: "Yarn Cache",
            description: "Yarn download cache. Re-downloads on demand.",
            risk_level: RiskLevel::Safe,
            detection: DetectionMethod::KnownPath { path: "~/Library/Caches/Yarn", expandable: true },
            cleanup: CleanupMethod::ShellCommand {
                command: "yarn cache clean".to_string(),
                run_in_terminal: false,
                refresh_after: false,
            },
        },
        CleanupPattern {
            id: "bun-cache",
            category: CleanupCategory::PackageManager,
            name: "Bun Cache",
            description: "Bun install cache. Re-downloads on demand.",
            risk_level: RiskLevel::Safe,
            detection: DetectionMethod::KnownPath { path: "~/.bun/install/cache", expandable: true },
            cleanup: CleanupMethod::ShellCommand {
                command: "bun pm cache rm".to_string(),
                run_in_terminal: false,
                refresh_after: false,
            },
        },
        CleanupPattern {
            id: "homebrew-cache",
            category: CleanupCategory::PackageManager,
            name: "Homebrew Cache",
            description: "Downloaded bottles and old formula versions.",
            risk_level: RiskLevel::Safe,
            detection: DetectionMethod::KnownPath { path: "~/Library/Caches/Homebrew", expandable: true },
            cleanup: CleanupMethod::ShellCommand {
                command: "brew cleanup --prune=all".to_string(),
                run_in_terminal: false,
                refresh_after: false,
            },
        },
        CleanupPattern {
            id: "pip-cache",
            category: CleanupCategory::PackageManager,
            name: "pip Cache",
            description: "Python package download cache.",
            risk_level: RiskLevel::Safe,
            detection: DetectionMethod::KnownPath { path: "~/Library/Caches/pip", expandable: true },
            cleanup: CleanupMethod::ShellCommand {
                command: "pip cache purge".to_string(),
                run_in_terminal: false,
                refresh_after: false,
            },
        },
        CleanupPattern {
            id: "go-modcache",
            category: CleanupCategory::PackageManager,
            name: "Go Module Cache",
            description: "Go module download cache. Re-downloads on demand.",
            risk_level: RiskLevel::Safe,
            detection: DetectionMethod::KnownPath { path: "~/go/pkg/mod", expandable: true },
            cleanup: CleanupMethod::ShellCommand {
                command: "go clean -modcache".to_string(),
                run_in_terminal: false,
                refresh_after: false,
            },
        },
        CleanupPattern {
            id: "cargo-registry",
            category: CleanupCategory::PackageManager,
            name: "Cargo Registry Cache",
            description: "Downloaded crate sources and indices. Re-downloads on demand.",
            risk_level: RiskLevel::Safe,
            detection: DetectionMethod::KnownPath { path: "~/.cargo/registry", expandable: true },
            cleanup: CleanupMethod::Delete { use_trash: true },
        },
        CleanupPattern {
            id: "cocoapods-cache",
            category: CleanupCategory::PackageManager,
            name: "CocoaPods Cache",
            description: "CocoaPods spec and download cache.",
            risk_level: RiskLevel::Safe,
            detection: DetectionMethod::KnownPath { path: "~/Library/Caches/CocoaPods", expandable: true },
            cleanup: CleanupMethod::ShellCommand {
                command: "pod cache clean --all".to_string(),
                run_in_terminal: false,
                refresh_after: false,
            },
        },
        CleanupPattern {
            id: "composer-cache",
            category: CleanupCategory::PackageManager,
            name: "Composer Cache",
            description: "PHP Composer download cache.",
            risk_level: RiskLevel::Safe,
            detection: DetectionMethod::KnownPath { path: "~/.cache/composer", expandable: true },
            cleanup: CleanupMethod::ShellCommand {
                command: "composer clear-cache".to_string(),
                run_in_terminal: false,
                refresh_after: false,
            },
        },
        CleanupPattern {
            id: "gem-cache",
            category: CleanupCategory::PackageManager,
            name: "Ruby Gems",
            description: "Old Ruby gem versions. Removes outdated gems.",
            risk_level: RiskLevel::Safe,
            detection: DetectionMethod::KnownPath { path: "~/.gem", expandable: true },
            cleanup: CleanupMethod::ShellCommand {
                command: "gem cleanup".to_string(),
                run_in_terminal: false,
                refresh_after: false,
            },
        },
    ]
}
```

**Step 3: Create containers.rs**

```rust
use super::*;

pub fn patterns() -> Vec<CleanupPattern> {
    vec![
        CleanupPattern {
            id: "docker-system",
            category: CleanupCategory::Container,
            name: "Docker (images/containers/volumes)",
            description: "Unused Docker images, stopped containers, and volumes. Re-pull images as needed.",
            risk_level: RiskLevel::Caution,
            detection: DetectionMethod::Command {
                check_cmd: "docker system df --format '{{.Size}}' 2>/dev/null | head -1",
                parse_hint: "docker_df",
            },
            cleanup: CleanupMethod::ShellCommand {
                command: "docker system prune -a --volumes -f".to_string(),
                run_in_terminal: true,
                refresh_after: false,
            },
        },
        CleanupPattern {
            id: "docker-raw",
            category: CleanupCategory::Container,
            name: "Docker Disk Image",
            description: "Docker Desktop virtual disk image. Resize in Docker Desktop settings.",
            risk_level: RiskLevel::Caution,
            detection: DetectionMethod::KnownPath {
                path: "~/Library/Containers/com.docker.docker/Data/vms/0/data/Docker.raw",
                expandable: true,
            },
            cleanup: CleanupMethod::OpenInFinder,
        },
        CleanupPattern {
            id: "orbstack-data",
            category: CleanupCategory::Container,
            name: "OrbStack Data",
            description: "OrbStack container and VM data. Review in OrbStack app.",
            risk_level: RiskLevel::Caution,
            detection: DetectionMethod::KnownPath {
                path: "~/Library/OrbStack",
                expandable: true,
            },
            cleanup: CleanupMethod::OpenInFinder,
        },
    ]
}
```

**Step 4: Create browsers.rs**

```rust
use super::*;

pub fn patterns() -> Vec<CleanupPattern> {
    vec![
        CleanupPattern {
            id: "chrome-cache",
            category: CleanupCategory::Browser,
            name: "Chrome Cache",
            description: "Google Chrome browser cache. Regenerates automatically.",
            risk_level: RiskLevel::Safe,
            detection: DetectionMethod::KnownPath {
                path: "~/Library/Caches/Google/Chrome",
                expandable: true,
            },
            cleanup: CleanupMethod::Delete { use_trash: true },
        },
        CleanupPattern {
            id: "safari-cache",
            category: CleanupCategory::Browser,
            name: "Safari Cache",
            description: "Safari browser cache. Regenerates automatically.",
            risk_level: RiskLevel::Safe,
            detection: DetectionMethod::KnownPath {
                path: "~/Library/Caches/com.apple.Safari",
                expandable: true,
            },
            cleanup: CleanupMethod::Delete { use_trash: true },
        },
        CleanupPattern {
            id: "firefox-cache",
            category: CleanupCategory::Browser,
            name: "Firefox Cache",
            description: "Firefox browser cache. Regenerates automatically.",
            risk_level: RiskLevel::Safe,
            detection: DetectionMethod::KnownPath {
                path: "~/Library/Caches/Firefox",
                expandable: true,
            },
            cleanup: CleanupMethod::Delete { use_trash: true },
        },
    ]
}
```

**Step 5: Create ides.rs**

```rust
use super::*;

pub fn patterns() -> Vec<CleanupPattern> {
    vec![
        CleanupPattern {
            id: "jetbrains-cache",
            category: CleanupCategory::IDE,
            name: "JetBrains Cache",
            description: "IntelliJ/WebStorm/etc. caches. Regenerates on next IDE launch.",
            risk_level: RiskLevel::Safe,
            detection: DetectionMethod::KnownPath {
                path: "~/Library/Caches/JetBrains",
                expandable: true,
            },
            cleanup: CleanupMethod::Delete { use_trash: true },
        },
        CleanupPattern {
            id: "jetbrains-logs",
            category: CleanupCategory::IDE,
            name: "JetBrains Logs",
            description: "JetBrains IDE log files.",
            risk_level: RiskLevel::Safe,
            detection: DetectionMethod::KnownPath {
                path: "~/Library/Logs/JetBrains",
                expandable: true,
            },
            cleanup: CleanupMethod::Delete { use_trash: true },
        },
        CleanupPattern {
            id: "vscode-cache",
            category: CleanupCategory::IDE,
            name: "VS Code Cache",
            description: "VS Code editor cache. Regenerates on next launch.",
            risk_level: RiskLevel::Safe,
            detection: DetectionMethod::KnownPath {
                path: "~/Library/Application Support/Code/Cache",
                expandable: true,
            },
            cleanup: CleanupMethod::Delete { use_trash: true },
        },
        CleanupPattern {
            id: "zed-cache",
            category: CleanupCategory::IDE,
            name: "Zed Cache",
            description: "Zed editor cache. Regenerates on next launch.",
            risk_level: RiskLevel::Safe,
            detection: DetectionMethod::KnownPath {
                path: "~/Library/Caches/dev.zed.Zed",
                expandable: true,
            },
            cleanup: CleanupMethod::Delete { use_trash: true },
        },
    ]
}
```

**Step 6: Create system.rs**

```rust
use super::*;

pub fn patterns() -> Vec<CleanupPattern> {
    vec![
        CleanupPattern {
            id: "core-spotlight",
            category: CleanupCategory::System,
            name: "CoreSpotlight Index",
            description: "Spotlight search index. Rebuilds automatically but takes time.",
            risk_level: RiskLevel::Caution,
            detection: DetectionMethod::KnownPath {
                path: "~/Library/Caches/com.apple.SpotlightIndex",
                expandable: true,
            },
            cleanup: CleanupMethod::ShellCommand {
                command: "sudo mdutil -E /".to_string(),
                run_in_terminal: true,
                refresh_after: false,
            },
        },
        CleanupPattern {
            id: "system-logs",
            category: CleanupCategory::System,
            name: "System Logs",
            description: "Application and system log files.",
            risk_level: RiskLevel::Safe,
            detection: DetectionMethod::KnownPath {
                path: "~/Library/Logs",
                expandable: true,
            },
            cleanup: CleanupMethod::Delete { use_trash: true },
        },
        CleanupPattern {
            id: "diagnostic-reports",
            category: CleanupCategory::System,
            name: "Crash Reports",
            description: "Application crash and diagnostic reports.",
            risk_level: RiskLevel::Safe,
            detection: DetectionMethod::KnownPath {
                path: "~/Library/Logs/DiagnosticReports",
                expandable: true,
            },
            cleanup: CleanupMethod::Delete { use_trash: true },
        },
        CleanupPattern {
            id: "time-machine-snapshots",
            category: CleanupCategory::System,
            name: "Time Machine Local Snapshots",
            description: "Local Time Machine snapshots. Safe if you have external backup.",
            risk_level: RiskLevel::Caution,
            detection: DetectionMethod::Command {
                check_cmd: "tmutil listlocalsnapshots / 2>/dev/null | grep -c 'com.apple' || echo 0",
                parse_hint: "count_lines",
            },
            cleanup: CleanupMethod::ShellCommand {
                command: "tmutil thinlocalsnapshots / 99999999999".to_string(),
                run_in_terminal: true,
                refresh_after: false,
            },
        },
        CleanupPattern {
            id: "trash",
            category: CleanupCategory::System,
            name: "Trash",
            description: "Files in trash bin. Permanently removes all trashed files.",
            risk_level: RiskLevel::Safe,
            detection: DetectionMethod::KnownPath {
                path: "~/.Trash",
                expandable: true,
            },
            cleanup: CleanupMethod::ShellCommand {
                command: "rm -rf ~/.Trash/*".to_string(),
                run_in_terminal: false,
                refresh_after: false,
            },
        },
        CleanupPattern {
            id: "downloads-old",
            category: CleanupCategory::System,
            name: "Downloads Folder",
            description: "Review and remove old downloaded files manually.",
            risk_level: RiskLevel::Warning,
            detection: DetectionMethod::KnownPath {
                path: "~/Downloads",
                expandable: true,
            },
            cleanup: CleanupMethod::OpenInFinder,
        },
    ]
}
```

**Step 7: Create cloud_storage.rs**

```rust
use super::*;

pub fn patterns() -> Vec<CleanupPattern> {
    vec![
        CleanupPattern {
            id: "icloud-cache",
            category: CleanupCategory::CloudStorage,
            name: "iCloud Drive Cache",
            description: "Locally cached iCloud files. Review before removing.",
            risk_level: RiskLevel::Warning,
            detection: DetectionMethod::KnownPath {
                path: "~/Library/Mobile Documents",
                expandable: true,
            },
            cleanup: CleanupMethod::OpenInFinder,
        },
        CleanupPattern {
            id: "dropbox-cache",
            category: CleanupCategory::CloudStorage,
            name: "Dropbox Cache",
            description: "Dropbox local cache. Regenerates automatically.",
            risk_level: RiskLevel::Safe,
            detection: DetectionMethod::KnownPath {
                path: "~/Dropbox/.dropbox.cache",
                expandable: true,
            },
            cleanup: CleanupMethod::Delete { use_trash: true },
        },
    ]
}
```

**Step 8: Create app_data.rs**

```rust
use super::*;

pub fn patterns() -> Vec<CleanupPattern> {
    vec![
        CleanupPattern {
            id: "orphaned-app-support",
            category: CleanupCategory::AppData,
            name: "Orphaned App Data",
            description: "Application Support data for apps no longer installed. Review before deleting.",
            risk_level: RiskLevel::Warning,
            detection: DetectionMethod::Command {
                check_cmd: "ls ~/Library/Application\\ Support/ 2>/dev/null | wc -l",
                parse_hint: "orphaned_apps",
            },
            cleanup: CleanupMethod::OpenInFinder,
        },
        CleanupPattern {
            id: "ios-backups",
            category: CleanupCategory::AppData,
            name: "iOS Device Backups",
            description: "Local iPhone/iPad backups. Delete if using iCloud backup.",
            risk_level: RiskLevel::Warning,
            detection: DetectionMethod::KnownPath {
                path: "~/Library/Application Support/MobileSync/Backup",
                expandable: true,
            },
            cleanup: CleanupMethod::OpenInFinder,
        },
        CleanupPattern {
            id: "mail-downloads",
            category: CleanupCategory::AppData,
            name: "Mail Attachment Cache",
            description: "Cached mail attachments. Re-downloads from server on demand.",
            risk_level: RiskLevel::Safe,
            detection: DetectionMethod::KnownPath {
                path: "~/Library/Containers/com.apple.mail/Data/Library/Mail Downloads",
                expandable: true,
            },
            cleanup: CleanupMethod::Delete { use_trash: true },
        },
    ]
}
```

**Step 9: Create media.rs**

```rust
use super::*;

pub fn patterns() -> Vec<CleanupPattern> {
    vec![
        CleanupPattern {
            id: "spotify-cache",
            category: CleanupCategory::Media,
            name: "Spotify Cache",
            description: "Spotify offline/streaming cache. Re-downloads as needed.",
            risk_level: RiskLevel::Safe,
            detection: DetectionMethod::KnownPath {
                path: "~/Library/Application Support/Spotify/PersistentCache",
                expandable: true,
            },
            cleanup: CleanupMethod::Delete { use_trash: true },
        },
        CleanupPattern {
            id: "adobe-cache",
            category: CleanupCategory::Media,
            name: "Adobe Cache",
            description: "Adobe Creative Cloud application data. Review before deleting.",
            risk_level: RiskLevel::Caution,
            detection: DetectionMethod::KnownPath {
                path: "~/Library/Application Support/Adobe",
                expandable: true,
            },
            cleanup: CleanupMethod::OpenInFinder,
        },
        CleanupPattern {
            id: "steam-apps",
            category: CleanupCategory::Media,
            name: "Steam Games",
            description: "Installed Steam games. Review and uninstall via Steam.",
            risk_level: RiskLevel::Warning,
            detection: DetectionMethod::KnownPath {
                path: "~/Library/Application Support/Steam/steamapps",
                expandable: true,
            },
            cleanup: CleanupMethod::OpenInFinder,
        },
    ]
}
```

**Step 10: Verify compilation**

```bash
cd src-tauri && cargo check
```

**Step 11: Commit**

```bash
git add src-tauri/src/cleanup_patterns/
git commit -m "feat: add all cleanup pattern definitions (40+ patterns across 9 categories)"
```

---

### Task 3: Scanner Integration (PathPattern matching)

**Files:**
- Modify: `src-tauri/src/scanner_tree.rs:96-248` (build_skeleton function)
- Modify: `src-tauri/src/scanner_tree.rs:19-94` (assemble_tree function)

**Step 1: Add cleanup_pattern_id to FlatNode in build_skeleton**

In `scanner_tree.rs`, the `FlatNode` struct (line 118) and `build_tree` function (line 226) need the new field.

Add to `FlatNode` struct at line 118:

```rust
struct FlatNode {
    name: String,
    path: String,
    is_hidden: bool,
    children_indices: Vec<usize>,
    cleanup_pattern_id: Option<String>,  // NEW
}
```

**Step 2: Add pattern matching to build_skeleton**

Import the cleanup pattern matching at the top of the file and modify `build_skeleton` to accept patterns and check them.

Add a new parameter to `build_skeleton`:

```rust
pub fn build_skeleton(
    root: &Path, cancel_token: &Arc<AtomicBool>,
    on_progress: &Channel<ScanProgress>, on_log: &Channel<ScanLogEntry>,
    dir_count: &mut u32, options: &ScanOptions, root_dev: Option<u64>,
    scan_pattern_map: &std::collections::HashMap<String, Vec<usize>>,
    all_patterns: &[crate::cleanup_patterns::CleanupPattern],
) -> Option<FileNode> {
```

In the loop where child directories are discovered (around line 189-216), after `let child_name = ...`:

```rust
// Check cleanup patterns
let child_path_as_path = std::path::Path::new(&child_str);
let cleanup_id = scan_pattern_map.get(&child_name)
    .and_then(|indices| {
        indices.iter().find_map(|&i| {
            if crate::cleanup_patterns::matches_path_pattern(&all_patterns[i], child_path_as_path) {
                Some(all_patterns[i].id.to_string())
            } else {
                None
            }
        })
    });
```

Pass `cleanup_id` to `FlatNode` construction at line 205:

```rust
nodes.push(FlatNode {
    name: child_name,
    path: child_str,
    is_hidden,
    children_indices: Vec::new(),
    cleanup_pattern_id: cleanup_id,
});
```

**Step 3: Propagate cleanup_pattern_id in build_tree**

In the `build_tree` function (line 226), pass it to `FileNode`:

```rust
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
```

**Step 4: Update assemble_tree to preserve cleanup_pattern_id**

In `assemble_tree` (line 77), the `FileNode` construction needs to copy `cleanup_pattern_id` from the skeleton:

```rust
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
    cleanup_pattern_id: skel.cleanup_pattern_id.clone(),
});
```

**Step 5: Update prune_tree to preserve cleanup_pattern_id**

In `prune_tree`, ensure `..node.clone()` already copies it (it does since we use `..node.clone()`). Verify the "others" node at line 317 has `cleanup_pattern_id: None` and the fallback at line 334 also has it.

**Step 6: Update scanner.rs call site**

In `scanner.rs`, where `build_skeleton` is called (around line 92), pass the new parameters:

```rust
let patterns = crate::cleanup_patterns::all_patterns();
let scan_map = crate::cleanup_patterns::build_scan_pattern_map(&patterns);

let skeleton = scanner_tree::build_skeleton(
    root_path, &cancel_token,
    &on_progress, &on_log,
    &mut dir_count, &options, root_dev,
    &scan_map, &patterns,
);
```

**Step 7: Verify compilation**

```bash
cd src-tauri && cargo check
```

**Step 8: Commit**

```bash
git add src-tauri/src/scanner_tree.rs src-tauri/src/scanner.rs
git commit -m "feat: integrate cleanup pattern detection into directory scanner"
```

---

### Task 4: Tauri Commands (scan_cleanup_recommendations, execute_cleanup_recommendation, get_cleanup_patterns)

**Files:**
- Modify: `src-tauri/src/commands.rs` (add 3 new commands)
- Modify: `src-tauri/src/lib.rs:6-22` (register new commands)

**Step 1: Add scan_cleanup_recommendations command**

In `commands.rs`, add:

```rust
use crate::cleanup_patterns::{
    self, CleanupRecommendation, CleanupScanProgress, CleanupResult,
    CleanupPatternInfo, CleanupMethod, DetectionMethod, RiskLevel,
};

#[tauri::command]
pub async fn scan_cleanup_recommendations(
    on_progress: Channel<CleanupScanProgress>,
) -> Result<Vec<CleanupRecommendation>, String> {
    let patterns = cleanup_patterns::all_patterns();
    let total = patterns.len();
    let mut recommendations = Vec::new();

    for (i, pattern) in patterns.iter().enumerate() {
        let _ = on_progress.send(CleanupScanProgress {
            current_pattern: pattern.name.to_string(),
            checked: i + 1,
            total,
        });

        match &pattern.detection {
            DetectionMethod::KnownPath { path, expandable } => {
                let full_path = if *expandable {
                    cleanup_patterns::expand_home(path)
                } else {
                    std::path::PathBuf::from(path)
                };

                if full_path.exists() {
                    let size = cleanup_patterns::dir_size(&full_path);
                    if size > 0 {
                        recommendations.push(CleanupRecommendation {
                            pattern_id: pattern.id.to_string(),
                            pattern_name: pattern.name.to_string(),
                            category: pattern.category.clone(),
                            risk_level: pattern.risk_level.clone(),
                            description: pattern.description.to_string(),
                            paths: vec![full_path.to_string_lossy().to_string()],
                            total_size: size,
                            cleanup_method: pattern.cleanup.clone(),
                        });
                    }
                }
            }
            DetectionMethod::Command { check_cmd, .. } => {
                if let Ok(output) = std::process::Command::new("sh")
                    .arg("-c")
                    .arg(check_cmd)
                    .output()
                {
                    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if !stdout.is_empty() && stdout != "0" {
                        recommendations.push(CleanupRecommendation {
                            pattern_id: pattern.id.to_string(),
                            pattern_name: pattern.name.to_string(),
                            category: pattern.category.clone(),
                            risk_level: pattern.risk_level.clone(),
                            description: pattern.description.to_string(),
                            paths: vec![],
                            total_size: 0, // Command-based patterns report size differently
                            cleanup_method: pattern.cleanup.clone(),
                        });
                    }
                }
            }
            DetectionMethod::PathPattern { .. } => {
                // PathPattern items are detected during scan, skip in standalone
            }
        }
    }

    // Sort by size descending
    recommendations.sort_by(|a, b| b.total_size.cmp(&a.total_size));
    Ok(recommendations)
}
```

**Step 2: Add execute_cleanup_recommendation command**

```rust
#[tauri::command]
pub async fn execute_cleanup_recommendation(
    pattern_id: String,
    paths: Vec<String>,
) -> Result<CleanupResult, String> {
    let patterns = cleanup_patterns::all_patterns();
    let pattern = patterns.iter()
        .find(|p| p.id == pattern_id)
        .ok_or_else(|| format!("Unknown pattern: {}", pattern_id))?;

    match &pattern.cleanup {
        CleanupMethod::Delete { use_trash } => {
            let mut total_freed: u64 = 0;
            for path_str in &paths {
                let path = std::path::Path::new(path_str);
                if !path.exists() { continue; }

                // Protected path check
                let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
                let canonical_str = canonical.to_string_lossy();
                for protected in &["/", "/System", "/usr", "/bin", "/sbin", "/Users"] {
                    if canonical_str == *protected {
                        return Err(format!("Cannot delete protected path: {}", path_str));
                    }
                }

                let size = cleanup_patterns::dir_size(path);
                if *use_trash {
                    trash::delete(path).map_err(|e| format!("Trash failed: {}", e))?;
                } else {
                    if path.is_dir() {
                        std::fs::remove_dir_all(path)
                            .map_err(|e| format!("Delete failed: {}", e))?;
                    } else {
                        std::fs::remove_file(path)
                            .map_err(|e| format!("Delete failed: {}", e))?;
                    }
                }
                total_freed += size;
            }
            Ok(CleanupResult {
                success: true,
                freed_bytes: total_freed,
                message: format!("Deleted {} item(s)", paths.len()),
            })
        }
        CleanupMethod::ShellCommand { command, run_in_terminal, .. } => {
            // Dangerous pattern check
            let lower = command.to_lowercase();
            for dp in DANGEROUS_PATTERNS {
                if lower.contains(dp) {
                    // Allow but log warning
                    break;
                }
            }

            if *run_in_terminal {
                execute_in_terminal(command)?;
                Ok(CleanupResult {
                    success: true,
                    freed_bytes: 0,
                    message: "Command sent to Terminal".to_string(),
                })
            } else {
                let output = std::process::Command::new("sh")
                    .arg("-c")
                    .arg(command)
                    .output()
                    .map_err(|e| format!("Command failed: {}", e))?;

                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();

                Ok(CleanupResult {
                    success: output.status.success(),
                    freed_bytes: 0,
                    message: if output.status.success() { stdout } else { stderr },
                })
            }
        }
        CleanupMethod::OpenInFinder => {
            if let Some(path) = paths.first() {
                open_in_finder_impl(path)?;
            }
            Ok(CleanupResult {
                success: true,
                freed_bytes: 0,
                message: "Opened in Finder".to_string(),
            })
        }
    }
}
```

**Step 3: Add get_cleanup_patterns command**

```rust
#[tauri::command]
pub fn get_cleanup_patterns() -> Vec<CleanupPatternInfo> {
    cleanup_patterns::all_patterns()
        .iter()
        .map(CleanupPatternInfo::from)
        .collect()
}
```

**Step 4: Register commands in lib.rs**

Add to the `invoke_handler` in `src-tauri/src/lib.rs`:

```rust
commands::scan_cleanup_recommendations,
commands::execute_cleanup_recommendation,
commands::get_cleanup_patterns,
```

**Step 5: Verify compilation**

```bash
cd src-tauri && cargo check
```

**Step 6: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat: add Tauri commands for cleanup scan, execution, and pattern listing"
```

---

### Task 5: Frontend Types & API & Store

**Files:**
- Modify: `src/lib/types.ts` (add cleanup types + FileNode field)
- Modify: `src/lib/api.ts` (add 3 API functions)
- Create: `src/lib/stores/cleanupRecommendationStore.ts`

**Step 1: Add types to types.ts**

Add `cleanup_pattern_id` to the `FileNode` interface and add new types:

```typescript
// Add to FileNode interface:
cleanup_pattern_id: string | null;

// Add new types:
export type CleanupCategory =
  | "DevTools" | "PackageManager" | "Container" | "Browser"
  | "IDE" | "System" | "CloudStorage" | "AppData" | "Media";

export type RiskLevel = "Safe" | "Caution" | "Warning";

export type CleanupMethod =
  | { Delete: { use_trash: boolean } }
  | { ShellCommand: { command: string; run_in_terminal: boolean; refresh_after: boolean } }
  | "OpenInFinder";

export interface CleanupRecommendation {
  pattern_id: string;
  pattern_name: string;
  category: CleanupCategory;
  risk_level: RiskLevel;
  description: string;
  paths: string[];
  total_size: number;
  cleanup_method: CleanupMethod;
}

export interface CleanupScanProgress {
  current_pattern: string;
  checked: number;
  total: number;
}

export interface CleanupResult {
  success: boolean;
  freed_bytes: number;
  message: string;
}

export interface CleanupPatternInfo {
  id: string;
  category: CleanupCategory;
  name: string;
  description: string;
  risk_level: RiskLevel;
  cleanup_method: CleanupMethod;
}
```

**Step 2: Add API functions to api.ts**

```typescript
import type { CleanupRecommendation, CleanupScanProgress, CleanupResult, CleanupPatternInfo } from "./types";

export async function scanCleanupRecommendations(
  onProgress: Channel<CleanupScanProgress>,
): Promise<CleanupRecommendation[]> {
  return invoke("scan_cleanup_recommendations", { onProgress });
}

export async function executeCleanupRecommendation(
  patternId: string,
  paths: string[],
): Promise<CleanupResult> {
  return invoke("execute_cleanup_recommendation", { patternId, paths });
}

export async function getCleanupPatterns(): Promise<CleanupPatternInfo[]> {
  return invoke("get_cleanup_patterns");
}
```

**Step 3: Create cleanupRecommendationStore.ts**

```typescript
import { writable } from "svelte/store";
import type { CleanupRecommendation, CleanupScanProgress } from "../types";

export const recommendations = writable<CleanupRecommendation[]>([]);
export const cleanupScanning = writable(false);
export const cleanupProgress = writable<CleanupScanProgress | null>(null);
export const selectedPatternIds = writable<Set<string>>(new Set());

export function resetCleanupRecommendations() {
  recommendations.set([]);
  cleanupScanning.set(false);
  cleanupProgress.set(null);
  selectedPatternIds.set(new Set());
}

export function removeRecommendation(patternId: string) {
  recommendations.update((recs) => recs.filter((r) => r.pattern_id !== patternId));
  selectedPatternIds.update((ids) => {
    ids.delete(patternId);
    return new Set(ids);
  });
}
```

**Step 4: Commit**

```bash
git add src/lib/types.ts src/lib/api.ts src/lib/stores/cleanupRecommendationStore.ts
git commit -m "feat: add frontend types, API, and store for cleanup recommendations"
```

---

### Task 6: Toolbar Redesign + MenuDropdown Component

**Files:**
- Modify: `src/lib/components/Toolbar.svelte`
- Create: `src/lib/components/MenuDropdown.svelte`

**Step 1: Create MenuDropdown.svelte**

A dropdown menu component with view toggles and settings links. Use existing toolbar button styling. Handle click-outside to close.

Props:
- All the toggle states and callbacks currently in Toolbar
- `onOpenCleanupSettings`, `onOpenSettings`

Sections:
- **View**: Tree Panel, Extensions (F8), Treemap (F9), separator, Free Space (F6), Unknown Space (F7), Treemap Options
- **separator**
- Cleanup Actions...
- Settings... (⌘,)

Each view toggle shows a checkmark when active. Keyboard shortcuts shown on the right.

**Step 2: Simplify Toolbar.svelte**

Remove: Free, Unknown, Tree, Extensions, Treemap, Cleanups, Settings, Options buttons.

Add: Cleanup button (new), Menu button (opens MenuDropdown).

New Toolbar layout:
```
[Open Folder] [Refresh] [Stop] | [Parent] [Child] | [Cleanup] | [≡ Menu ▾]
```

Pass `onOpenCleanupRecommendations` callback prop for the Cleanup button.

**Step 3: Verify frontend builds**

```bash
pnpm build
```

**Step 4: Commit**

```bash
git add src/lib/components/Toolbar.svelte src/lib/components/MenuDropdown.svelte
git commit -m "feat: simplify toolbar and add menu dropdown for view/settings"
```

---

### Task 7: CleanupRecommendations Modal

**Files:**
- Create: `src/lib/components/CleanupRecommendations.svelte`
- Create: `src/lib/components/CleanupCategoryGroup.svelte`
- Create: `src/lib/components/CleanupRecommendationItem.svelte`
- Create: `src/lib/components/CleanupConfirmDialog.svelte`

**Step 1: Create CleanupConfirmDialog.svelte**

Simple confirmation dialog with:
- Pattern name and description
- Risk level indicator
- Paths to be cleaned
- Cancel / Confirm buttons
- For Caution: yellow border. For Warning: red border.

**Step 2: Create CleanupRecommendationItem.svelte**

Single recommendation row:
- Checkbox (disabled for Warning items)
- Risk level icon (● green, ⚠ yellow, ⛔ red)
- Pattern name
- Size (formatted with formatSize utility)
- Risk label
- [Clean] button
- Expandable path list for multi-path items (node_modules)

Props: `recommendation: CleanupRecommendation`, `selected: boolean`, `onToggle`, `onClean`

**Step 3: Create CleanupCategoryGroup.svelte**

Collapsible category group:
- Header with category name, total size, [Clean All] button
- List of CleanupRecommendationItem children
- Toggle expand/collapse

Props: `category: string`, `items: CleanupRecommendation[]`, `selectedIds: Set<string>`

**Step 4: Create CleanupRecommendations.svelte**

Main modal component:
- Header with title, [Scan] re-scan button, [✕] close
- ProgressBar during scan
- Summary line ("38.7 GB reclaimable across 12 items")
- Category groups (sorted by total size)
- Footer: selected total + [Clean Selected Items] button
- On mount: call `scanCleanupRecommendations` via Channel API
- Handle cleanup execution with confirmation for Caution+ items

**Step 5: Verify frontend builds**

```bash
pnpm build
```

**Step 6: Commit**

```bash
git add src/lib/components/CleanupRecommendations.svelte src/lib/components/CleanupCategoryGroup.svelte src/lib/components/CleanupRecommendationItem.svelte src/lib/components/CleanupConfirmDialog.svelte
git commit -m "feat: add CleanupRecommendations modal with category grouping and execution"
```

---

### Task 8: Wire Modal into App.svelte

**Files:**
- Modify: `src/App.svelte`

**Step 1: Add state and import**

```typescript
import CleanupRecommendations from "./lib/components/CleanupRecommendations.svelte";

let showCleanupRecommendations = $state(false);
```

**Step 2: Add modal mount**

Near the existing CleanupSettings modal mount (around line 387):

```svelte
{#if showCleanupRecommendations}
  <CleanupRecommendations onClose={() => showCleanupRecommendations = false} />
{/if}
```

**Step 3: Pass callback to Toolbar**

Add `onOpenCleanupRecommendations={() => showCleanupRecommendations = true}` to Toolbar props.

**Step 4: Verify with dev server**

```bash
pnpm tauri dev
```

**Step 5: Commit**

```bash
git add src/App.svelte
git commit -m "feat: wire CleanupRecommendations modal into main app"
```

---

### Task 9: TreeView Cleanup Icon

**Files:**
- Modify: `src/lib/components/TreeRow.svelte`

**Step 1: Add cleanup icon to row**

In the name cell rendering (around line 92), after the folder/file icon, add:

```svelte
{#if node.cleanup_pattern_id}
  <span
    class="cleanup-badge"
    title="Cleanable: {node.cleanup_pattern_id}"
    onclick|stopPropagation={() => onCleanupClick?.(node)}
  >🧹</span>
{/if}
```

**Step 2: Add styling**

```css
.cleanup-badge {
  font-size: 12px;
  cursor: pointer;
  margin-left: 2px;
  opacity: 0.7;
}
.cleanup-badge:hover {
  opacity: 1;
}
```

**Step 3: Add callback prop and wire to parent**

Add `onCleanupClick` optional prop. In TreeView.svelte, handle it by showing a context menu or directly executing cleanup for that pattern.

**Step 4: Commit**

```bash
git add src/lib/components/TreeRow.svelte src/lib/components/TreeView.svelte
git commit -m "feat: add cleanup badge icon to TreeView rows"
```

---

### Task 10: ContextMenu Cleanup Integration

**Files:**
- Modify: `src/lib/components/ContextMenu.svelte`

**Step 1: Add cleanup recommendation action**

In the menu items, after existing cleanup actions section (around line 225), add:

```svelte
{#if node?.cleanup_pattern_id}
  <div class="separator"></div>
  <button onclick={() => handleRecommendedCleanup(node)}>
    🧹 Clean: {node.cleanup_pattern_id}
  </button>
{/if}
```

**Step 2: Implement handleRecommendedCleanup**

```typescript
async function handleRecommendedCleanup(node: FileNode) {
  const patternId = node.cleanup_pattern_id;
  if (!patternId) return;

  const ok = await confirm(`Clean "${node.name}" (${formatSize(node.size)})?`, { title: "Cleanup" });
  if (!ok) return;

  try {
    const result = await executeCleanupRecommendation(patternId, [node.path]);
    if (result.success) {
      await refreshSubtree(node.path);
    }
  } catch (e) {
    window.alert(`Cleanup failed: ${e}`);
  }
  onClose();
}
```

**Step 3: Commit**

```bash
git add src/lib/components/ContextMenu.svelte
git commit -m "feat: add cleanup recommendation action to context menu"
```

---

### Task 11: Treemap Hatching Overlay

**Files:**
- Modify: `src/lib/components/Treemap.svelte`
- Modify: `src/lib/components/settings/TreemapTab.svelte` (add toggle option)
- Modify: `src/lib/stores/treemapOptionsStore.ts` (add showCleanupOverlay option)

**Step 1: Add option to treemapOptionsStore**

Add `showCleanupOverlay: boolean` (default: true) to the options.

**Step 2: Add toggle to TreemapTab settings**

Add a checkbox: "Show cleanup overlay" in the treemap options panel.

**Step 3: Add hatching render pass to Treemap**

After the main treemap render, if `showCleanupOverlay` is enabled, iterate treemap leaves and for those whose source `FileNode` has `cleanup_pattern_id`, draw a diagonal hatching pattern overlay:

```typescript
function renderCleanupOverlay(ctx: CanvasRenderingContext2D, nodes: TreemapNode[]) {
  ctx.save();
  ctx.strokeStyle = "rgba(255, 255, 255, 0.3)";
  ctx.lineWidth = 1;

  for (const node of nodes) {
    if (!node.data.cleanup_pattern_id) continue;
    const { x0, y0, x1, y1 } = node;
    ctx.beginPath();
    // Draw diagonal lines
    const step = 6;
    for (let i = 0; i < (x1 - x0) + (y1 - y0); i += step) {
      const startX = Math.max(x0, x0 + i - (y1 - y0));
      const startY = Math.min(y1, y0 + i);
      const endX = Math.min(x1, x0 + i);
      const endY = Math.max(y0, y0 + i - (x1 - x0));
      ctx.moveTo(startX, startY);
      ctx.lineTo(endX, endY);
    }
    ctx.stroke();
  }
  ctx.restore();
}
```

**Step 4: Add cleanup info to treemap tooltip**

When hovering a node with `cleanup_pattern_id`, append to tooltip: " (cleanable)"

**Step 5: Commit**

```bash
git add src/lib/components/Treemap.svelte src/lib/components/settings/TreemapTab.svelte src/lib/stores/treemapOptionsStore.ts
git commit -m "feat: add hatching overlay for cleanable items in treemap"
```

---

### Task 12: Integration Testing & Polish

**Step 1: Test full scan with pattern detection**

```bash
pnpm tauri dev
```

- Open a folder that contains `node_modules` and/or `target/` directories
- Verify 🧹 icons appear in TreeView
- Verify hatching appears in Treemap
- Right-click a tagged item, verify "Clean" option appears
- Test cleanup execution

**Step 2: Test standalone scan**

- Click Cleanup button in toolbar
- Verify progress bar shows during scan
- Verify categories appear sorted by size
- Verify risk levels display correctly
- Test individual item cleanup
- Test "Clean Selected Items" batch cleanup

**Step 3: Test menu dropdown**

- Click Menu button
- Toggle each view option, verify panel shows/hides
- Click "Cleanup Actions...", verify existing modal opens
- Click "Settings...", verify settings modal opens
- Verify keyboard shortcuts still work (F5-F9, ⌘,)

**Step 4: Edge cases to test**

- Scan with no cleanable items found
- Docker not installed (command detection should gracefully skip)
- Permission denied paths
- Empty trash
- Cancel mid-cleanup-scan

**Step 5: Final commit**

```bash
git add -A
git commit -m "feat: cleanup recommendations - integration polish and fixes"
```
