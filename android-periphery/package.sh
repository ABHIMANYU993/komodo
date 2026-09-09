#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
NDK_BIN="/home/icebyte/work/android-ndk-r26d/toolchains/llvm/prebuilt/linux-x86_64/bin"
TARGET="aarch64-linux-android"
RELEASE_BIN="$SCRIPT_DIR/target/$TARGET/release/komodo-android-periphery"
DIST_DIR="$SCRIPT_DIR/dist"
MAGISK_BUILD_DIR="$DIST_DIR/magisk_build"
ZIP_NAME="komodo-android-periphery-v0.1.0.zip"

echo "=== 1. Building release binary for $TARGET ==="
export PATH="$HOME/.cargo/bin:$NDK_BIN:$PATH"
cd "$SCRIPT_DIR"
cargo build --release --target "$TARGET"

echo "=== 2. Stripping debug symbols ==="
mkdir -p "$MAGISK_BUILD_DIR"
cp "$RELEASE_BIN" "$MAGISK_BUILD_DIR/komodo-android-periphery"
"$NDK_BIN/llvm-strip" --strip-all "$MAGISK_BUILD_DIR/komodo-android-periphery"

echo "=== 3. Staging Magisk module files ==="
cp "$SCRIPT_DIR/magisk/module.prop" "$MAGISK_BUILD_DIR/"
cp "$SCRIPT_DIR/magisk/service.sh" "$MAGISK_BUILD_DIR/"
cp "$SCRIPT_DIR/magisk/customize.sh" "$MAGISK_BUILD_DIR/"
cp "$SCRIPT_DIR/magisk/config.toml.example" "$MAGISK_BUILD_DIR/"
chmod +x "$MAGISK_BUILD_DIR/service.sh"
chmod +x "$MAGISK_BUILD_DIR/customize.sh"
chmod +x "$MAGISK_BUILD_DIR/komodo-android-periphery"

echo "=== 4. Packaging Magisk zip: $DIST_DIR/$ZIP_NAME ==="
rm -f "$DIST_DIR/$ZIP_NAME"
(cd "$MAGISK_BUILD_DIR" && zip -r "$DIST_DIR/$ZIP_NAME" .)
rm -rf "$MAGISK_BUILD_DIR"

echo "=== Successfully built $DIST_DIR/$ZIP_NAME ==="
ls -lh "$DIST_DIR/$ZIP_NAME"
