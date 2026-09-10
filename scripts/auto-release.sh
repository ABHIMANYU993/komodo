#!/usr/bin/env bash
# Komodo Git Post-Commit Release Automation
# Automatically compiles release binaries, packages, and container images on local 16 cores

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

VERSION=$(grep -m1 '^version = ' "$ROOT_DIR/Cargo.toml" | cut -d'"' -f2)
echo ">>> [Auto-Release] Triggered for version v$VERSION"

"$ROOT_DIR/scripts/build-release-local.sh" "v$VERSION"

echo ">>> [Auto-Release] Local multi-core build complete for v$VERSION"
