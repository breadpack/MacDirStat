use super::*;

pub fn patterns() -> Vec<CleanupPattern> {
    vec![
        CleanupPattern {
            id: "spotify-cache",
            category: CleanupCategory::Media,
            name: "Spotify Cache",
            description: "Spotify streaming cache for offline playback. Safe to delete; music is re-cached as you listen.",
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
            name: "Adobe Application Data",
            description: "Adobe application support data and caches. Review before deleting; may contain presets and settings.",
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
            description: "Installed Steam games. Can be very large. Review in Finder and uninstall games via Steam.",
            risk_level: RiskLevel::Warning,
            detection: DetectionMethod::KnownPath {
                path: "~/Library/Application Support/Steam/steamapps",
                expandable: true,
            },
            cleanup: CleanupMethod::OpenInFinder,
        },
    ]
}
