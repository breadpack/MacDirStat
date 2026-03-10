use super::*;

pub fn patterns() -> Vec<CleanupPattern> {
    vec![
        CleanupPattern {
            id: "icloud-cache",
            category: CleanupCategory::CloudStorage,
            name: "iCloud Mobile Documents",
            description: "iCloud Drive local cache. Review in Finder; removing files may affect iCloud sync.",
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
            description: "Dropbox local cache files. Safe to delete; Dropbox will rebuild its cache.",
            risk_level: RiskLevel::Safe,
            detection: DetectionMethod::KnownPath {
                path: "~/Dropbox/.dropbox.cache",
                expandable: true,
            },
            cleanup: CleanupMethod::Delete { use_trash: true },
        },
    ]
}
