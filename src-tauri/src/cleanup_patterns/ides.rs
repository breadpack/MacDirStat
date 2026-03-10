use super::*;

pub fn patterns() -> Vec<CleanupPattern> {
    vec![
        CleanupPattern {
            id: "jetbrains-cache",
            category: CleanupCategory::IDE,
            name: "JetBrains IDE Cache",
            description: "Cache files for JetBrains IDEs (IntelliJ, WebStorm, etc.). Safe to delete; rebuilt on next launch.",
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
            name: "JetBrains IDE Logs",
            description: "Log files for JetBrains IDEs. Safe to delete; new logs are created on next launch.",
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
            description: "Visual Studio Code cache data. Safe to delete; rebuilt on next launch.",
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
            name: "Zed Editor Cache",
            description: "Zed editor cache data. Safe to delete; rebuilt on next launch.",
            risk_level: RiskLevel::Safe,
            detection: DetectionMethod::KnownPath {
                path: "~/Library/Caches/dev.zed.Zed",
                expandable: true,
            },
            cleanup: CleanupMethod::Delete { use_trash: true },
        },
    ]
}
