use super::*;

pub fn patterns() -> Vec<CleanupPattern> {
    vec![
        CleanupPattern {
            id: "orphaned-app-support",
            category: CleanupCategory::AppData,
            name: "Application Support Folders",
            description: "Application Support directory entries. Some may belong to uninstalled apps. Review in Finder.",
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
            description: "Local backups of iPhones and iPads. Can be very large. Review before deleting.",
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
            name: "Mail Downloads",
            description: "Attachments downloaded from Apple Mail. Safe to delete; can be re-downloaded from mail server.",
            risk_level: RiskLevel::Safe,
            detection: DetectionMethod::KnownPath {
                path: "~/Library/Containers/com.apple.mail/Data/Library/Mail Downloads",
                expandable: true,
            },
            cleanup: CleanupMethod::Delete { use_trash: true },
        },
    ]
}
