use super::*;

pub fn patterns() -> Vec<CleanupPattern> {
    vec![
        CleanupPattern {
            id: "core-spotlight",
            category: CleanupCategory::System,
            name: "Spotlight Index Cache",
            description: "Spotlight search index cache. Requires sudo to rebuild. macOS will re-index automatically.",
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
            name: "User Log Files",
            description: "Application and system log files in your user library. Safe to delete.",
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
            name: "Diagnostic Reports",
            description: "Crash reports and diagnostic logs. Safe to delete; new reports are generated as needed.",
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
            description: "Local Time Machine snapshots stored on disk. Thinning removes old snapshots to free space.",
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
            description: "Files in the Trash. Permanently deletes all items in Trash to reclaim disk space.",
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
            description: "Your Downloads folder. Review contents in Finder and remove files you no longer need.",
            risk_level: RiskLevel::Warning,
            detection: DetectionMethod::KnownPath {
                path: "~/Downloads",
                expandable: true,
            },
            cleanup: CleanupMethod::OpenInFinder,
        },
    ]
}
