use super::*;

/// Shell script that checks Docker Desktop installation and running state,
/// starts it if needed, runs the cleanup command, then restarts to reclaim disk.
const DOCKER_CLEANUP_SCRIPT: &str = r#"
# Check if Docker Desktop is installed
if [ ! -d "/Applications/Docker.app" ]; then
    echo "Docker Desktop is not installed. Nothing to clean."
    exit 0
fi

# Check if Docker daemon is running
WAS_RUNNING=false
if docker info >/dev/null 2>&1; then
    WAS_RUNNING=true
    echo "Docker Desktop is running. Pruning..."
else
    echo "Docker Desktop is not running. Starting..."
    open -a Docker
    for i in $(seq 1 30); do
        if docker info >/dev/null 2>&1; then
            echo "Docker Desktop started."
            break
        fi
        sleep 2
    done
    if ! docker info >/dev/null 2>&1; then
        echo "Error: Docker Desktop failed to start within 60 seconds."
        exit 1
    fi
fi

echo "Running docker system prune..."
docker system prune -a --volumes -f

# Restart Docker to reclaim disk image space
echo ""
echo "Restarting Docker Desktop to reclaim disk space..."
osascript -e 'quit app "Docker"'
sleep 3

if [ "$WAS_RUNNING" = true ]; then
    open -a Docker
    echo "Done. Docker Desktop is restarting to shrink the disk image."
else
    echo "Done. Docker Desktop has been stopped."
fi

echo "If disk image is still large, reduce limit in:"
echo "  Docker Desktop > Settings > Resources > Disk image size"
"#;

pub fn patterns() -> Vec<CleanupPattern> {
    vec![
        CleanupPattern {
            id: "docker-system",
            category: CleanupCategory::Container,
            name: "Docker System Data",
            description: "Docker images, containers, and volumes. Starts Docker Desktop if needed, prunes all unused data, then restarts to reclaim disk.",
            risk_level: RiskLevel::Caution,
            detection: DetectionMethod::Command {
                check_cmd: "[ -d /Applications/Docker.app ] && docker system df --format '{{.Size}}' 2>/dev/null | head -1",
                parse_hint: "docker_df",
            },
            cleanup: CleanupMethod::ShellCommand {
                command: DOCKER_CLEANUP_SCRIPT.to_string(),
                run_in_terminal: false,
                refresh_after: false,
            },
        },
        CleanupPattern {
            id: "docker-raw",
            category: CleanupCategory::Container,
            name: "Docker Desktop Disk Image",
            description: "Docker Desktop VM disk image. Prunes unused data and restarts Docker Desktop to reclaim disk space.",
            risk_level: RiskLevel::Caution,
            detection: DetectionMethod::KnownPath {
                path: "~/Library/Containers/com.docker.docker/Data/vms/0/data/Docker.raw",
                expandable: true,
            },
            cleanup: CleanupMethod::ShellCommand {
                command: DOCKER_CLEANUP_SCRIPT.to_string(),
                run_in_terminal: false,
                refresh_after: false,
            },
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
