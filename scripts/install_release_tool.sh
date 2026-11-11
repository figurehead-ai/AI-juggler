#!/usr/bin/env bash
# AJ-E2007: installs a pinned, prebuilt supply-chain tool for CI and the
# release jobs: the project's own release binary, checked against its
# SHA-256, so nothing is compiled and the version can't drift. Prints the
# directory to put on PATH.
#
#   install_release_tool.sh TOOL DEST_DIR
#
# TOOL: cargo-vet, cargo-auditable, cargo-cyclonedx, cargo-machete or zizmor. To move to a new
# version, take each asset's digest from its GitHub release
# (`gh api repos/OWNER/REPO/releases --jq '.[0].assets[] | "\(.name) \(.digest)"'`).
set -euo pipefail

tool="$1"
dest="$2"
platform="$(uname -s)-$(uname -m)"
case "$tool:$platform" in
  cargo-vet:Linux-x86_64)
    url=https://github.com/mozilla/cargo-vet/releases/download/v0.10.3/cargo-vet-x86_64-unknown-linux-gnu.tar.xz
    sha256=e83878b80913fbe43629c28768e14ab51f4e86a4ba8f9dd08cc25b8dbc503c19 ;;
  cargo-vet:Darwin-arm64)
    url=https://github.com/mozilla/cargo-vet/releases/download/v0.10.3/cargo-vet-aarch64-apple-darwin.tar.xz
    sha256=a83f7d6f48a069890743ba61f7637660c8577f263efd64fee47e673632f0e423 ;;
  cargo-auditable:Darwin-arm64)
    url=https://github.com/rust-secure-code/cargo-auditable/releases/download/v0.7.7/cargo-auditable-aarch64-apple-darwin.tar.xz
    sha256=a66ad6392861ca1abfe0b02b522622d91edf63e5d7b60f4d3088b5c23d240c72 ;;
  cargo-auditable:Darwin-x86_64)
    url=https://github.com/rust-secure-code/cargo-auditable/releases/download/v0.7.7/cargo-auditable-x86_64-apple-darwin.tar.xz
    sha256=4b87f06ea13d32466bbc68b1f5715ac11753a052d9935245380bddce8a3f0e50 ;;
  cargo-cyclonedx:Darwin-arm64)
    url=https://github.com/CycloneDX/cyclonedx-rust-cargo/releases/download/cargo-cyclonedx-0.5.9/cargo-cyclonedx-aarch64-apple-darwin.tar.xz
    sha256=4c53dfa21e70b65bf7f8d2592aadde3bcb02c1a40b6ec63b877e5ca65a29e180 ;;
  cargo-cyclonedx:Darwin-x86_64)
    url=https://github.com/CycloneDX/cyclonedx-rust-cargo/releases/download/cargo-cyclonedx-0.5.9/cargo-cyclonedx-x86_64-apple-darwin.tar.xz
    sha256=59d2a583fa632f8759456c1b531340331255b277386d23c598a3dbbc916fde63 ;;
  cargo-machete:Linux-x86_64)
    url=https://github.com/bnjbvr/cargo-machete/releases/download/v0.9.2/cargo-machete-v0.9.2-x86_64-unknown-linux-musl.tar.gz
    sha256=48200087f54c55aabcd4db4af1e25742b49846c02a1b1bfa134711945b35b2e9 ;;
  cargo-machete:Darwin-arm64)
    url=https://github.com/bnjbvr/cargo-machete/releases/download/v0.9.2/cargo-machete-v0.9.2-aarch64-apple-darwin.tar.gz
    sha256=63e28fee386d82d33f2d12406c857f98e2d4697f3f7df7f71f34dff07fca0fde ;;
  zizmor:Linux-x86_64)
    url=https://github.com/zizmorcore/zizmor/releases/download/v1.32.0/zizmor-x86_64-unknown-linux-gnu.tar.gz
    sha256=bb110cc765eca9cd7efb2e284a956be5974c60027b0970f16397d86208a9957b ;;
  zizmor:Darwin-arm64)
    url=https://github.com/zizmorcore/zizmor/releases/download/v1.32.0/zizmor-aarch64-apple-darwin.tar.gz
    sha256=6efed18c4539a663fd3c851a2be9533fd8456c4e646d9416206da059154a0d3c ;;
  *) echo "::error::no pinned $tool for $platform" >&2; exit 1 ;;
esac

mkdir -p "$dest"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
archive="$work/${url##*/}"
curl --proto '=https' --tlsv1.2 -fsSL "$url" -o "$archive"
if command -v sha256sum >/dev/null; then
  got="$(sha256sum "$archive" | cut -d' ' -f1)"
else
  got="$(shasum -a 256 "$archive" | cut -d' ' -f1)"
fi
if [ "$got" != "$sha256" ]; then
  echo "::error::$tool: digest $got, expected $sha256" >&2
  exit 1
fi
tar -xf "$archive" -C "$work"
binary="$(find "$work" -type f -name "$tool" -perm -u+x | head -1)"
[ -n "$binary" ] || { echo "::error::$tool: no executable in $url" >&2; exit 1; }
mv "$binary" "$dest/$tool"
echo "$dest"
