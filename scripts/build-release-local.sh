#!/usr/bin/env bash
# Komodo Fast Multi-Core Local Release Builder & Publisher
# Uses all host CPU cores to build release binaries, packages, and container images locally
# Repository: https://github.com/ABHIMANYU993/komodo

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
CORES=$(nproc 2>/dev/null || echo 4)
VERSION="${1:-v2.4.2}"
VER_NUM="${VERSION#v}"
RELEASE_DIST="$ROOT_DIR/release-dist"

echo "================================================================="
echo "  Komodo Multi-Core Release & Container Builder"
echo "  Version: $VERSION ($VER_NUM) | CPU Cores: $CORES"
echo "================================================================="

cd "$ROOT_DIR"

# 1. Build Host Rust Binaries (core, periphery, km) using all CPU cores
echo ">>> [1/6] Compiling host binaries (core, periphery, km) with $CORES threads..."
cargo build --release -p komodo_core -p komodo_periphery -p komodo_cli -j "$CORES"

# 2. Build Statically Linked Musl Periphery (runs on Alpine / all Linux distros)
echo ">>> [2/6] Compiling static musl periphery with $CORES threads..."
cargo build --release -p komodo_periphery --target x86_64-unknown-linux-musl -j "$CORES"

# 3. Build Android Periphery (aarch64-linux-android) and Magisk Package
echo ">>> [3/6] Packaging Android Periphery and Magisk module..."
"$ROOT_DIR/android-periphery/package.sh"

# 4. Strip binaries and stage artifacts
echo ">>> [4/6] Staging release artifacts into $RELEASE_DIST..."
rm -rf "$RELEASE_DIST"
mkdir -p "$RELEASE_DIST"

cp "$ROOT_DIR/target/release/core" "$RELEASE_DIST/core-x86_64"
cp "$ROOT_DIR/target/release/periphery" "$RELEASE_DIST/periphery-x86_64"
cp "$ROOT_DIR/target/x86_64-unknown-linux-musl/release/periphery" "$RELEASE_DIST/periphery-x86_64-musl"
cp "$ROOT_DIR/target/release/km" "$RELEASE_DIST/km-x86_64"

strip "$RELEASE_DIST/core-x86_64" "$RELEASE_DIST/periphery-x86_64" "$RELEASE_DIST/km-x86_64" 2>/dev/null || true

# Copy Android artifacts
cp "$ROOT_DIR/android-periphery/target/aarch64-linux-android/release/komodo-android-periphery" "$RELEASE_DIST/komodo-android-periphery"
if [ -d "/home/icebyte/work/android-ndk-r26d/toolchains/llvm/prebuilt/linux-x86_64/bin" ]; then
    /home/icebyte/work/android-ndk-r26d/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-strip --strip-all "$RELEASE_DIST/komodo-android-periphery" 2>/dev/null || true
fi

cp "$ROOT_DIR/android-periphery/dist/komodo-android-periphery.zip" "$RELEASE_DIST/"
cp "$ROOT_DIR/android-periphery/dist/komodo-android-periphery-v${VER_NUM}.zip" "$RELEASE_DIST/" 2>/dev/null || true
cp -f "$RELEASE_DIST/periphery-x86_64" "$RELEASE_DIST/periphery"

# Compress core-x86_64 (106MB raw exceeds GitHub 100MB single-file limit)
echo ">>> Compressing core-x86_64 into core-x86_64.tar.gz..."
tar -czf "$RELEASE_DIST/core-x86_64.tar.gz" -C "$RELEASE_DIST" core-x86_64
rm -f "$RELEASE_DIST/core-x86_64"

# Compute Checksums
cd "$RELEASE_DIST"
sha256sum * > SHA256SUMS.txt
echo "Staged Artifacts:"
ls -lh "$RELEASE_DIST"
cd "$ROOT_DIR"

# 5. Build Local Docker Container Images with latest, 2, and version tags
if command -v docker >/dev/null 2>&1; then
    echo ">>> [5/6] Building local container images with tags: $VER_NUM, 2, latest..."

    # 1. Scratch binaries image
    echo ">>> Packaging local binaries into scratch image..."
    mkdir -p /tmp/binaries-ctx
    cp target/release/core /tmp/binaries-ctx/core
    cp target/release/periphery /tmp/binaries-ctx/periphery
    cp target/release/km /tmp/binaries-ctx/km
    printf 'FROM scratch\nCOPY core /core\nCOPY periphery /periphery\nCOPY km /km\n' > /tmp/binaries-ctx/Dockerfile
    docker build -t "ghcr.io/abhimanyu993/komodo-binaries:$VER_NUM" \
        -t "ghcr.io/abhimanyu993/komodo-binaries:2" \
        -t "ghcr.io/abhimanyu993/komodo-binaries:latest" /tmp/binaries-ctx
    rm -rf /tmp/binaries-ctx

    # 2. Komodo Periphery Image
    echo ">>> Building ghcr.io/abhimanyu993/komodo-periphery..."
    docker build -f bin/periphery/single-arch.Dockerfile \
        --build-arg "BINARIES_IMAGE=ghcr.io/abhimanyu993/komodo-binaries:2" \
        -t "ghcr.io/abhimanyu993/komodo-periphery:$VER_NUM" \
        -t "ghcr.io/abhimanyu993/komodo-periphery:2" \
        -t "ghcr.io/abhimanyu993/komodo-periphery:latest" .

    # 3. Komodo Android Periphery Image
    echo ">>> Building ghcr.io/abhimanyu993/komodo-android-periphery..."
    cd "$ROOT_DIR/android-periphery"
    docker build -f Dockerfile --build-arg "VERSION=$VER_NUM" \
        -t "ghcr.io/abhimanyu993/komodo-android-periphery:$VER_NUM" \
        -t "ghcr.io/abhimanyu993/komodo-android-periphery:2" \
        -t "ghcr.io/abhimanyu993/komodo-android-periphery:latest" .
    cd "$ROOT_DIR"

    # 4. Komodo Core Image (builds UI assets in node builder, bundles core & km)
    echo ">>> Building ghcr.io/abhimanyu993/komodo-core..."
    docker build -f bin/core/single-arch.Dockerfile \
        --build-arg "BINARIES_IMAGE=ghcr.io/abhimanyu993/komodo-binaries:2" \
        -t "ghcr.io/abhimanyu993/komodo-core:$VER_NUM" \
        -t "ghcr.io/abhimanyu993/komodo-core:2" \
        -t "ghcr.io/abhimanyu993/komodo-core:latest" .

    echo ">>> Local Docker images successfully built:"
    docker images | grep "abhimanyu993/komodo" || true
else
    echo ">>> [5/6] Docker not found, skipping container image build."
fi

echo "================================================================="
echo "  [6/6] Release binaries & container images built successfully!"
echo "================================================================="
