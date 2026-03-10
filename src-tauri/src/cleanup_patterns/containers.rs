use super::*;

pub fn patterns() -> Vec<CleanupPattern> {
    vec![
        CleanupPattern {
            id: "docker-system",
            category: CleanupCategory::Container,
            name: "Docker System Data",
            description: "Docker images, containers, and volumes. Prunes all unused Docker data.",
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
            name: "Docker Desktop Disk Image",
            description: "Docker Desktop VM disk image. Can grow very large. Review in Finder before taking action.",
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
            description: "OrbStack container and VM data. Review before deleting as it contains all container data.",
            risk_level: RiskLevel::Caution,
            detection: DetectionMethod::KnownPath {
                path: "~/Library/OrbStack",
                expandable: true,
            },
            cleanup: CleanupMethod::OpenInFinder,
        },
    ]
}
