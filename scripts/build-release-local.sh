#!/usr/bin/env bash
# Komodo Fast Multi-Core Local Release Builder & Publisher
# Uses all host CPU cores to build release binaries and packages locally
# Repository: https://github.com/ABHIMANYU993/komodo

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
CORES=$(nproc 2>/dev/null || echo 4)
VERSION="${1:-v2.4.1}"
RELEASE_DIST="$ROOT_DIR/release-dist"

echo "================================================================="
echo "  Komodo Multi-Core Release Builder"
echo "  Version: $VERSION | CPU Cores: $CORES"
echo "================================================================="

cd "$ROOT_DIR"

# 1. Build Host Rust Binaries (core, periphery, km) using all CPU cores
echo ">>> [1/5] Compiling host binaries (core, periphery, km) with $CORES threads..."
cargo build --release -p komodo_core -p komodo_periphery -p komodo_cli -j "$CORES"

# 2. Build Statically Linked Musl Periphery (runs on Alpine / all Linux distros)
echo ">>> [2/5] Compiling static musl periphery with $CORES threads..."
cargo build --release -p komodo_periphery --target x86_64-unknown-linux-musl -j "$CORES"

# 3. Build Android Periphery (aarch64-linux-android) and Magisk Package
echo ">>> [3/5] Packaging Android Periphery and Magisk module..."
"$ROOT_DIR/android-periphery/package.sh"

# 4. Strip binaries and stage artifacts
echo ">>> [4/5] Staging release artifacts into $RELEASE_DIST..."
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
cp "$ROOT_DIR/android-periphery/dist/komodo-android-periphery-v${VERSION#v}.zip" "$RELEASE_DIST/" 2>/dev/null || true
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

echo "================================================================="
echo "  Release artifacts successfully built and verified!"
echo "================================================================="
