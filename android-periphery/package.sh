#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
NDK_BIN="${ANDROID_NDK_HOME:-/home/icebyte/work/android-ndk-r26d}/toolchains/llvm/prebuilt/linux-x86_64/bin"
TARGET="aarch64-linux-android"
RELEASE_BIN="$SCRIPT_DIR/target/$TARGET/release/komodo-android-periphery"
DIST_DIR="$SCRIPT_DIR/dist"
MAGISK_BUILD_DIR="$DIST_DIR/magisk_build"
VERSION=$(grep '^version' "$SCRIPT_DIR/Cargo.toml" | head -n 1 | cut -d'"' -f2)

ZIP_CANONICAL="komodo-android-periphery.zip"
ZIP_VERSIONED="komodo-android-periphery-v${VERSION}.zip"

echo "=== 1. Building release binary for $TARGET (v${VERSION}) ==="
export PATH="$HOME/.cargo/bin:$NDK_BIN:$PATH"
cd "$SCRIPT_DIR"
cargo build --release --target "$TARGET"

echo "=== 2. Stripping debug symbols ==="
rm -rf "$MAGISK_BUILD_DIR"
mkdir -p "$MAGISK_BUILD_DIR"
cp "$RELEASE_BIN" "$MAGISK_BUILD_DIR/komodo-android-periphery"
"$NDK_BIN/llvm-strip" --strip-all "$MAGISK_BUILD_DIR/komodo-android-periphery"

echo "=== 3. Staging Magisk module files ==="
cp "$SCRIPT_DIR/magisk/module.prop" "$MAGISK_BUILD_DIR/"
cp "$SCRIPT_DIR/magisk/service.sh" "$MAGISK_BUILD_DIR/"
cp "$SCRIPT_DIR/magisk/customize.sh" "$MAGISK_BUILD_DIR/"
cp "$SCRIPT_DIR/magisk/uninstall.sh" "$MAGISK_BUILD_DIR/"
cp "$SCRIPT_DIR/magisk/komodo-control" "$MAGISK_BUILD_DIR/"
cp "$SCRIPT_DIR/magisk/config.toml.example" "$MAGISK_BUILD_DIR/"

chmod +x "$MAGISK_BUILD_DIR/service.sh"
chmod +x "$MAGISK_BUILD_DIR/customize.sh"
chmod +x "$MAGISK_BUILD_DIR/uninstall.sh"
chmod +x "$MAGISK_BUILD_DIR/komodo-control"
chmod +x "$MAGISK_BUILD_DIR/komodo-android-periphery"

echo "=== 4. Packaging Magisk zip: $DIST_DIR/$ZIP_CANONICAL ==="
mkdir -p "$DIST_DIR"
rm -f "$DIST_DIR/$ZIP_CANONICAL" "$DIST_DIR/$ZIP_VERSIONED"
rm -f "$DIST_DIR/${ZIP_CANONICAL}.sha256" "$DIST_DIR/${ZIP_VERSIONED}.sha256"

(cd "$MAGISK_BUILD_DIR" && zip -r "$DIST_DIR/$ZIP_CANONICAL" .)
cp "$DIST_DIR/$ZIP_CANONICAL" "$DIST_DIR/$ZIP_VERSIONED"

# Generate SHA256 checksums (pure checksum filename format)
(cd "$DIST_DIR" && sha256sum "$ZIP_CANONICAL" > "${ZIP_CANONICAL}.sha256")
(cd "$DIST_DIR" && sha256sum "$ZIP_VERSIONED" > "${ZIP_VERSIONED}.sha256")

rm -rf "$MAGISK_BUILD_DIR"

echo "=== Successfully built release artifacts ==="
ls -lh "$DIST_DIR"
cat "$DIST_DIR/${ZIP_CANONICAL}.sha256"
