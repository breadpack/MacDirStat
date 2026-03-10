use super::*;

pub fn patterns() -> Vec<CleanupPattern> {
    vec![
        CleanupPattern {
            id: "chrome-cache",
            category: CleanupCategory::Browser,
            name: "Chrome Cache",
            description: "Google Chrome browser cache. Safe to delete; Chrome rebuilds its cache automatically.",
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
            description: "Safari browser cache. Safe to delete; Safari rebuilds its cache automatically.",
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
            description: "Firefox browser cache. Safe to delete; Firefox rebuilds its cache automatically.",
            risk_level: RiskLevel::Safe,
            detection: DetectionMethod::KnownPath {
                path: "~/Library/Caches/Firefox",
                expandable: true,
            },
            cleanup: CleanupMethod::Delete { use_trash: true },
        },
    ]
}
