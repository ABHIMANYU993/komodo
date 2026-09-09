# ─────────────────────────────────────────────────────────────
# komodo-android-periphery Docker image
#
# This image is a distribution artifact — it holds:
#   - The aarch64 native Android periphery binary (pre-built in CI)
#   - The Magisk module assets for deployment on Android devices
#
# The binary must be pre-built for aarch64-linux-android before
# this Dockerfile is invoked. The CI workflow handles this via
# the Rust cross-compilation step before docker buildx build.
#
# Usage (extract binary from image):
#   docker create --name tmp ghcr.io/abhimanyu993/komodo-android-periphery:2
#   docker cp tmp:/android-periphery/komodo-android-periphery ./
#   docker rm tmp
# ─────────────────────────────────────────────────────────────

FROM debian:trixie-slim

ARG VERSION="2.4.0"

WORKDIR /android-periphery

# Copy the pre-built aarch64 binary
COPY target/aarch64-linux-android/release/komodo-android-periphery ./komodo-android-periphery

# Copy Magisk module scaffolding
COPY magisk/ ./magisk/

# Copy example config
COPY config.toml.example ./config.toml.example

RUN chmod +x ./komodo-android-periphery

LABEL org.opencontainers.image.title="Komodo Android Periphery" \
      org.opencontainers.image.description="Native aarch64 Komodo Periphery daemon for rooted Android devices (Magisk). Version ${VERSION}." \
      org.opencontainers.image.licenses="GPL-3.0" \
      org.opencontainers.image.version="${VERSION}"

CMD ["/bin/bash"]
