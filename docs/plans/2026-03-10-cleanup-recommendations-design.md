# Cleanup Recommendations Feature Design

## Overview

MacDirStat에 macOS 디스크 용량 확보를 위한 자동 감지 + 추천 + 실행 기능을 추가한다.

- **스캔 연계형**: 스캔 중 트리 내에서 정리 가능한 패턴을 자동 감지하여 TreeView/Treemap에 표시
- **독립 도우미형**: 별도 모달에서 macOS 전체를 대상으로 알려진 정리 패턴을 검사하고 결과 표시
- **실행 지원**: 삭제, CLI 명령어 실행, Finder 열기까지 앱 내에서 직접 수행

## 1. Pattern Model (Rust)

### CleanupPattern

```rust
pub struct CleanupPattern {
    pub id: &'static str,
    pub category: CleanupCategory,
    pub name: &'static str,
    pub description: &'static str,
    pub risk_level: RiskLevel,
    pub detection: DetectionMethod,
    pub cleanup: CleanupMethod,
}
```

### DetectionMethod

```rust
pub enum DetectionMethod {
    /// Scan-time path pattern matching (dynamic detection within tree)
    PathPattern {
        dir_name: Option<&'static str>,
        path_contains: Option<&'static str>,
        parent_marker: Option<&'static str>,
    },
    /// Known fixed path (for standalone scan)
    KnownPath {
        path: &'static str,
        expandable: bool,
    },
    /// System command execution (docker system df, tmutil, etc.)
    Command {
        check_cmd: &'static str,
        parse_hint: &'static str,
    },
}
```

### CleanupMethod

```rust
pub enum CleanupMethod {
    Delete { use_trash: bool },
    ShellCommand {
        command: &'static str,
        run_in_terminal: bool,
        refresh_after: bool,
    },
    OpenInFinder,
}
```

### Enums

```rust
pub enum CleanupCategory {
    DevTools,        // Xcode, Cargo, Gradle, Maven, Android
    PackageManager,  // npm, pnpm, yarn, Homebrew, pip, go, etc.
    Container,       // Docker, OrbStack
    Browser,         // Chrome, Safari, Firefox
    IDE,             // JetBrains, VS Code, Zed
    System,          // Logs, CoreSpotlight, Time Machine, Trash
    CloudStorage,    // iCloud, Dropbox cache
    AppData,         // Orphaned apps, iOS backups, Mail
    Media,           // Spotify, Adobe, Steam
}

pub enum RiskLevel {
    Safe,     // Auto-regenerated, no functional impact
    Caution,  // Requires re-download/rebuild, time cost
    Warning,  // Potential data loss, must confirm
}
```

## 2. Built-in Pattern Registry

Patterns are organized into category submodules. Adding a new pattern requires only adding a struct to the relevant module.

### Module Structure

```
src-tauri/src/cleanup_patterns/
├── mod.rs               // all_patterns(), matching logic, types
├── dev_tools.rs
├── package_managers.rs
├── containers.rs
├── browsers.rs
├── ides.rs
├── system.rs
├── cloud_storage.rs
├── app_data.rs
└── media.rs
```

### Pattern List

#### DevTools
| ID | Name | Detection | Risk | Cleanup |
|---|---|---|---|---|
| `xcode-derived-data` | Xcode DerivedData | KnownPath `~/Library/Developer/Xcode/DerivedData` | Safe | Delete(trash) |
| `xcode-archives` | Xcode Archives | KnownPath `~/Library/Developer/Xcode/Archives` | Caution | OpenInFinder |
| `xcode-device-support` | Xcode Device Support | KnownPath `~/Library/Developer/Xcode/iOS DeviceSupport` | Safe | Delete(trash) |
| `xcode-simulators` | iOS Simulators (unavailable) | Command `xcrun simctl list` | Safe | Shell `xcrun simctl delete unavailable` |
| `xcode-spm-cache` | Swift PM Cache | KnownPath `~/Library/Caches/org.swift.swiftpm` | Safe | Delete(trash) |
| `cargo-target` | Rust build artifacts | PathPattern dir=`target`, parent_marker=`Cargo.toml` | Safe | Delete(trash) |
| `gradle-cache` | Gradle Cache | KnownPath `~/.gradle/caches` | Safe | Delete(trash) |
| `maven-repo` | Maven Local Repository | KnownPath `~/.m2/repository` | Safe | Delete(trash) |
| `android-sdk` | Android SDK/Emulators | KnownPath `~/Library/Android/sdk` | Caution | OpenInFinder |

#### PackageManager
| ID | Name | Detection | Risk | Cleanup |
|---|---|---|---|---|
| `node-modules` | node_modules | PathPattern dir=`node_modules`, parent_marker=`package.json` | Safe | Delete(permanent) |
| `npm-cache` | npm Cache | KnownPath `~/.npm` | Safe | Shell `npm cache clean --force` |
| `pnpm-store` | pnpm Store | KnownPath `~/Library/pnpm/store` | Safe | Shell `pnpm store prune` |
| `yarn-cache` | Yarn Cache | KnownPath `~/Library/Caches/Yarn` | Safe | Shell `yarn cache clean` |
| `bun-cache` | Bun Cache | KnownPath `~/.bun/install/cache` | Safe | Shell `bun pm cache rm` |
| `homebrew-cache` | Homebrew Cache | KnownPath `~/Library/Caches/Homebrew` | Safe | Shell `brew cleanup --prune=all` |
| `pip-cache` | pip Cache | KnownPath `~/Library/Caches/pip` | Safe | Shell `pip cache purge` |
| `go-modcache` | Go Module Cache | KnownPath `~/go/pkg/mod` | Safe | Shell `go clean -modcache` |
| `cargo-registry` | Cargo Registry Cache | KnownPath `~/.cargo/registry` | Safe | Delete(trash) |
| `cocoapods-cache` | CocoaPods Cache | KnownPath `~/Library/Caches/CocoaPods` | Safe | Shell `pod cache clean --all` |
| `composer-cache` | Composer Cache | KnownPath `~/.cache/composer` | Safe | Shell `composer clear-cache` |
| `gem-cache` | Ruby Gems | KnownPath `~/.gem` | Safe | Shell `gem cleanup` |

#### Container
| ID | Name | Detection | Risk | Cleanup |
|---|---|---|---|---|
| `docker-system` | Docker (images/containers/volumes) | Command `docker system df` | Caution | Shell `docker system prune -a --volumes -f` (terminal) |
| `docker-raw` | Docker Disk Image | KnownPath `~/Library/Containers/com.docker.docker/Data/vms/0/data/Docker.raw` | Caution | OpenInFinder |
| `orbstack-data` | OrbStack Data | KnownPath `~/Library/OrbStack` | Caution | OpenInFinder |

#### Browser
| ID | Name | Detection | Risk | Cleanup |
|---|---|---|---|---|
| `chrome-cache` | Chrome Cache | KnownPath `~/Library/Caches/Google/Chrome` | Safe | Delete(trash) |
| `safari-cache` | Safari Cache | KnownPath `~/Library/Caches/com.apple.Safari` | Safe | Delete(trash) |
| `firefox-cache` | Firefox Cache | KnownPath `~/Library/Caches/Firefox` | Safe | Delete(trash) |

#### IDE
| ID | Name | Detection | Risk | Cleanup |
|---|---|---|---|---|
| `jetbrains-cache` | JetBrains Cache | KnownPath `~/Library/Caches/JetBrains` | Safe | Delete(trash) |
| `jetbrains-logs` | JetBrains Logs | KnownPath `~/Library/Logs/JetBrains` | Safe | Delete(trash) |
| `vscode-cache` | VS Code Cache | KnownPath `~/Library/Application Support/Code/Cache` | Safe | Delete(trash) |
| `zed-cache` | Zed Cache | KnownPath `~/Library/Caches/dev.zed.Zed` | Safe | Delete(trash) |

#### System
| ID | Name | Detection | Risk | Cleanup |
|---|---|---|---|---|
| `core-spotlight` | CoreSpotlight Index | KnownPath `~/Library/Caches/com.apple.SpotlightIndex` | Caution | Shell `mdutil -E /` (terminal) |
| `system-logs` | System Logs | KnownPath `~/Library/Logs` | Safe | Delete(trash) |
| `diagnostic-reports` | Crash Reports | KnownPath `~/Library/Logs/DiagnosticReports` | Safe | Delete(trash) |
| `time-machine-snapshots` | Time Machine Local Snapshots | Command `tmutil listlocalsnapshots /` | Caution | Shell `tmutil thinlocalsnapshots / 99999999999` (terminal) |
| `trash` | Trash | KnownPath `~/.Trash` | Safe | Shell `rm -rf ~/.Trash/*` |
| `downloads-old` | Old Downloads (90d+) | KnownPath `~/Downloads` | Warning | OpenInFinder |

#### CloudStorage
| ID | Name | Detection | Risk | Cleanup |
|---|---|---|---|---|
| `icloud-cache` | iCloud Drive Cache | KnownPath `~/Library/Mobile Documents` | Warning | OpenInFinder |
| `dropbox-cache` | Dropbox Cache | KnownPath `~/Dropbox/.dropbox.cache` | Safe | Delete(trash) |

#### AppData
| ID | Name | Detection | Risk | Cleanup |
|---|---|---|---|---|
| `orphaned-app-support` | Orphaned App Data | Command (compare app list) | Warning | OpenInFinder |
| `ios-backups` | iOS Backups | KnownPath `~/Library/Application Support/MobileSync/Backup` | Warning | OpenInFinder |
| `mail-downloads` | Mail Attachments | KnownPath `~/Library/Containers/com.apple.mail/Data/Library/Mail Downloads` | Safe | Delete(trash) |

#### Media
| ID | Name | Detection | Risk | Cleanup |
|---|---|---|---|---|
| `spotify-cache` | Spotify Cache | KnownPath `~/Library/Application Support/Spotify/PersistentCache` | Safe | Delete(trash) |
| `adobe-cache` | Adobe Cache | KnownPath `~/Library/Application Support/Adobe` | Caution | OpenInFinder |
| `steam-apps` | Steam Games | KnownPath `~/Library/Application Support/Steam/steamapps` | Warning | OpenInFinder |

### Adding a New Pattern

To add a new pattern, add an entry to the relevant category module's `patterns()` function:

```rust
// Example: adding a new pattern to dev_tools.rs
CleanupPattern {
    id: "my-new-pattern",
    category: CleanupCategory::DevTools,
    name: "My Tool Cache",
    description: "Cache for MyTool. Safe to delete, regenerates on next run.",
    risk_level: RiskLevel::Safe,
    detection: DetectionMethod::KnownPath {
        path: "~/Library/Caches/MyTool",
        expandable: true,
    },
    cleanup: CleanupMethod::Delete { use_trash: true },
}
```

No other code changes required. The pattern is automatically included in both scan-time detection and standalone scan.

## 3. Backend Architecture

### Scan Integration

Scanner checks `PathPattern` type patterns during directory traversal:

```rust
fn check_scan_patterns(path: &Path, dir_name: &str) -> Option<&'static str> {
    // Pre-built HashMap<dir_name, Vec<PatternRef>> for O(1) lookup
    SCAN_PATTERN_MAP.get(dir_name)
        .and_then(|candidates| candidates.iter()
            .find(|p| p.matches(path))
            .map(|p| p.id))
}
```

- Matched pattern ID is stored in `FileNode.cleanup_pattern_id`
- Option to skip scanning subdirectories of matched patterns (node_modules, target/)
- Performance impact: negligible (HashMap lookup on dir_name string)

### FileNode Extension

```rust
pub struct FileNode {
    // ... existing fields
    pub cleanup_pattern_id: Option<String>,
}
```

### New Tauri Commands (3)

```rust
#[tauri::command]
async fn scan_cleanup_recommendations(
    on_progress: Channel<CleanupScanProgress>,
) -> Result<Vec<CleanupRecommendation>, String>

#[tauri::command]
async fn execute_cleanup_recommendation(
    pattern_id: String,
    paths: Vec<String>,
) -> Result<CleanupResult, String>

#[tauri::command]
fn get_cleanup_patterns() -> Vec<CleanupPatternInfo>
```

### Response Models

```rust
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

pub struct CleanupScanProgress {
    pub current_pattern: String,
    pub checked: usize,
    pub total: usize,
}

pub struct CleanupResult {
    pub success: bool,
    pub freed_bytes: u64,
    pub message: String,
}
```

### Safety

- Reuses existing dangerous command blocking from `execute_cleanup`
- Warning-level items always require confirmation dialog
- Path validation: symlink resolution, protected path checks
- Logging of all cleanup actions (pattern, paths, result)

## 4. Frontend UI

### 4.1 Toolbar Redesign

**Keep in toolbar** (frequent use):
- Open Folder, Refresh, Stop, Parent, Child, **Cleanup** (new)

**Move to Menu dropdown**:
- View toggles: Tree, Extensions, Treemap, Free, Unknown, Treemap Options
- Tools: Cleanup Actions..., Settings...

**Result:**
```
[Open Folder] [Refresh] [Stop] Scanning... | [Parent] [Child] | [Cleanup] | [Menu ▾]
```

**Menu dropdown:**
```
┌──────────────────────────┐
│  View                    │
│    ☑ Tree Panel          │
│    ☑ Extensions     F8   │
│    ☑ Treemap        F9   │
│    ─────────────────     │
│    ☐ Free Space     F6   │
│    ☐ Unknown Space  F7   │
│    ☐ Treemap Options     │
│  ─────────────────────── │
│  Cleanup Actions...      │
│  Settings...       ⌘,    │
└──────────────────────────┘
```

Keyboard shortcuts (F5-F9, ⌘,) remain unchanged.

### 4.2 CleanupRecommendations Modal

Triggered by toolbar Cleanup button. Auto-runs `scan_cleanup_recommendations` on open.

```
┌─────────────────────────────────────────────────────┐
│  Cleanup Recommendations                [Scan] [✕]  │
├─────────────────────────────────────────────────────┤
│  ProgressBar (during scan)                          │
├─────────────────────────────────────────────────────┤
│  Summary: 38.7 GB reclaimable across 12 items       │
├─────────────────────────────────────────────────────┤
│  ▾ DevTools (15.2 GB)                    [Clean All]│
│    ☑ ● Xcode DerivedData     2.4 GB    Safe  [Clean]│
│    ☑ ● iOS Simulators        8.1 GB    Safe  [Clean]│
│    ☑ ● Cargo build artifacts 4.7 GB    Safe  [Clean]│
│                                                     │
│  ▾ Container (44.9 GB)                   [Clean All]│
│    ☐ ⚠ Docker System        44.9 GB  Caution [Clean]│
│                                                     │
│  ▸ PackageManager (8.3 GB)               [Clean All]│
│  ▸ Browser (2.1 GB)                      [Clean All]│
├─────────────────────────────────────────────────────┤
│  Selected: 28.3 GB          [Clean Selected Items]  │
└─────────────────────────────────────────────────────┘
```

- Categories sorted by total size, collapsible
- Risk indicators: ● Safe (green), ⚠ Caution (yellow), ⛔ Warning (red)
- Default check state: Safe=checked, Caution=unchecked, Warning=disabled
- Multi-path items (node_modules) expandable for individual selection
- Confirmation dialog for Caution+ items

### 4.3 TreeView Integration

- Cleanup icon on rows with `cleanup_pattern_id`
- Hover tooltip: "Xcode DerivedData - Safe to delete (2.4 GB)"
- Click icon → context menu with "Clean this item" action
- Right-click ContextMenu shows "Cleanup: {pattern_name}" for matching items

### 4.4 Treemap Integration

- Hatching overlay on regions with `cleanup_pattern_id`
- Preserves existing color scheme (pattern overlay, not color change)
- Toggle on/off in Treemap Options panel
- Hover tooltip shows cleanup info

### 4.5 New Store

```typescript
// src/lib/stores/cleanupRecommendationStore.ts
interface CleanupRecommendationState {
    recommendations: CleanupRecommendation[];
    scanning: boolean;
    progress: CleanupScanProgress | null;
    selectedIds: Set<string>;
}
```

### 4.6 New Components

```
src/lib/components/
├── CleanupRecommendations.svelte
├── CleanupCategoryGroup.svelte
├── CleanupRecommendationItem.svelte
├── CleanupConfirmDialog.svelte
└── MenuDropdown.svelte
```

## 5. Data Flow Summary

### Flow 1: Scan-time Detection

```
scan_directory() → Scanner visits dirs → check_scan_patterns()
  → FileNode.cleanup_pattern_id set → sent to frontend
  → TreeView: icon display / Treemap: hatching overlay
  → User clicks → execute_cleanup_recommendation()
  → Result + optional subtree refresh
```

### Flow 2: Standalone Scan (Modal)

```
Toolbar Cleanup → modal opens → scan_cleanup_recommendations()
  → KnownPath: expand ~ → check existence → calculate size
  → Command: execute → parse output
  → Channel progress updates
  → User selects items → Clean Selected
  → execute_cleanup_recommendation() per item
  → Update list (remove/resize) + optional tree refresh
```

### Flow 3: Menu Dropdown

```
Menu button → dropdown with View toggles + Settings links
  → View toggles: immediate state change
  → Cleanup Actions...: opens existing CleanupSettings modal
  → Settings...: opens existing SettingsDialog modal
```

## 6. Change Summary

| Component | Change |
|---|---|
| Rust model | `FileNode` + `cleanup_pattern_id` field |
| Rust new module | `cleanup_patterns/` (9 submodules + mod.rs) |
| Rust commands | +3 (`scan_cleanup_recommendations`, `execute_cleanup_recommendation`, `get_cleanup_patterns`) |
| Scanner | PathPattern matching during traversal |
| Frontend components | +5 (CleanupRecommendations, CategoryGroup, RecommendationItem, ConfirmDialog, MenuDropdown) |
| Frontend store | +1 (cleanupRecommendationStore) |
| Toolbar | Simplified (6 buttons + menu), Cleanup button added |
| TreeView | Icon display + context menu extension |
| Treemap | Hatching overlay + options toggle |
| API | +3 functions |
