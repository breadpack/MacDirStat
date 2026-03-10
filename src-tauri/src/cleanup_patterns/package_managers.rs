use super::*;

pub fn patterns() -> Vec<CleanupPattern> {
    vec![
        CleanupPattern {
            id: "node-modules",
            category: CleanupCategory::PackageManager,
            name: "node_modules Directories",
            description: "Node.js dependency folders in projects. Safe to delete; restored with npm/pnpm/yarn install.",
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
            description: "Cached npm packages. Safe to clean; packages are re-downloaded as needed.",
            risk_level: RiskLevel::Safe,
            detection: DetectionMethod::KnownPath {
                path: "~/.npm",
                expandable: true,
            },
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
            description: "pnpm content-addressable package store. Safe to prune; unused packages are removed.",
            risk_level: RiskLevel::Safe,
            detection: DetectionMethod::KnownPath {
                path: "~/Library/pnpm/store",
                expandable: true,
            },
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
            description: "Cached Yarn packages. Safe to clean; packages are re-downloaded as needed.",
            risk_level: RiskLevel::Safe,
            detection: DetectionMethod::KnownPath {
                path: "~/Library/Caches/Yarn",
                expandable: true,
            },
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
            description: "Cached Bun packages. Safe to clean; packages are re-downloaded as needed.",
            risk_level: RiskLevel::Safe,
            detection: DetectionMethod::KnownPath {
                path: "~/.bun/install/cache",
                expandable: true,
            },
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
            description: "Cached Homebrew downloads and old versions. Safe to clean with brew cleanup.",
            risk_level: RiskLevel::Safe,
            detection: DetectionMethod::KnownPath {
                path: "~/Library/Caches/Homebrew",
                expandable: true,
            },
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
            description: "Cached Python pip packages. Safe to purge; packages are re-downloaded as needed.",
            risk_level: RiskLevel::Safe,
            detection: DetectionMethod::KnownPath {
                path: "~/Library/Caches/pip",
                expandable: true,
            },
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
            description: "Cached Go module downloads. Safe to clean; modules are re-downloaded as needed.",
            risk_level: RiskLevel::Safe,
            detection: DetectionMethod::KnownPath {
                path: "~/go/pkg/mod",
                expandable: true,
            },
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
            description: "Cached Rust crate downloads. Safe to delete; crates are re-downloaded on next build.",
            risk_level: RiskLevel::Safe,
            detection: DetectionMethod::KnownPath {
                path: "~/.cargo/registry",
                expandable: true,
            },
            cleanup: CleanupMethod::Delete { use_trash: true },
        },
        CleanupPattern {
            id: "cocoapods-cache",
            category: CleanupCategory::PackageManager,
            name: "CocoaPods Cache",
            description: "Cached CocoaPods specs and pods. Safe to clean; re-downloaded on next pod install.",
            risk_level: RiskLevel::Safe,
            detection: DetectionMethod::KnownPath {
                path: "~/Library/Caches/CocoaPods",
                expandable: true,
            },
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
            description: "Cached PHP Composer packages. Safe to clear; packages are re-downloaded as needed.",
            risk_level: RiskLevel::Safe,
            detection: DetectionMethod::KnownPath {
                path: "~/.cache/composer",
                expandable: true,
            },
            cleanup: CleanupMethod::ShellCommand {
                command: "composer clear-cache".to_string(),
                run_in_terminal: false,
                refresh_after: false,
            },
        },
        CleanupPattern {
            id: "gem-cache",
            category: CleanupCategory::PackageManager,
            name: "Ruby Gems Cache",
            description: "Cached Ruby gems. Safe to clean up old versions with gem cleanup.",
            risk_level: RiskLevel::Safe,
            detection: DetectionMethod::KnownPath {
                path: "~/.gem",
                expandable: true,
            },
            cleanup: CleanupMethod::ShellCommand {
                command: "gem cleanup".to_string(),
                run_in_terminal: false,
                refresh_after: false,
            },
        },
    ]
}
