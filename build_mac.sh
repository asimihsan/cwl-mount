#!/usr/bin/env bash

# References
# - https://github.com/walles/riff/blob/82f77c8/release.sh#L121-L136

set -euxo pipefail

BASEDIR=$(realpath "$(dirname "$0")")

CROSSBUILD_MACOS_SDK="macosx12.0"

# If this fails, try "xcodebuild -showsdks" to find one that exists
if ! xcrun -sdk $CROSSBUILD_MACOS_SDK --show-sdk-path >/dev/null; then
  echo >&2
  echo >&2 "ERROR: $CROSSBUILD_MACOS_SDK not found, try \"xcodebuild -showsdks\" to find a better one, then update build_mac.sh and try again"
  exit 1
fi

cargo install cargo-lipo

# Build macOS binaries
# TODO support ARM
# targets="aarch64-apple-darwin x86_64-apple-darwin"
targets="x86_64-apple-darwin"
for target in $targets; do
  rustup target add "$target"

  # From: https://stackoverflow.com/a/66875783/473672
  (SDKROOT=$(xcrun -sdk "$CROSSBUILD_MACOS_SDK" --show-sdk-path) \
  MACOSX_DEPLOYMENT_TARGET=$(xcrun -sdk "$CROSSBUILD_MACOS_SDK" --show-sdk-platform-version) \
    cd "$BASEDIR"/src && \
    cargo build --workspace --profile production --target="$target")
done

# From: https://developer.apple.com/documentation/apple-silicon/building-a-universal-macos-binary#Update-the-Architecture-List-of-Custom-Makefiles
#   src/target/aarch64-apple-darwin/production/cwl-mount
# TODO support ARM
lipo -create \
  -output "$BASEDIR"/src/target/cwl-universal-apple-darwin-release \
  "$BASEDIR"/src/target/x86_64-apple-darwin/production/cwl-mount

rsync -av "$BASEDIR"/src/target/cwl-universal-apple-darwin-release "$BASEDIR"/pkg/cwl-mount
(cd "$BASEDIR"/pkg && tar -czvf cwl-mount-0.1.1-darwin-x64_64.tar.gz cwl-mount)
rm -f "$BASEDIR"/pkg/cwl-mount