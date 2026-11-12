#!/usr/bin/env bash
# Leaves only the dependencies in a Cargo profile folder before CI saves it
# as a cache (Swatinem/rust-cache's rule, by hand: GitHub-owned actions
# only). The workspace's own crates rebuild on every checkout anyway (new
# files), so their artifacts, the test and app binaries, the incremental
# state and the app bundle are only cache weight: a 6 GB cache took 4
# minutes to restore (ADR-0025). Kept: every dependency's library, build
# script output and fingerprint, and the proc-macro libraries (.so,
# .dylib, .dll), which the next build loads.
#
#   scripts/ci/cache_keep_deps.sh <profile folder>...
#   e.g. target/debug, target/release, target/x86_64-apple-darwin/debug
#
# A folder that doesn't exist is skipped (a job that built less).
set -euo pipefail
[ "$#" -gt 0 ] || { echo "usage: $0 <profile folder>..." >&2; exit 2; }

for dir in "$@"; do
  if [ ! -d "$dir" ]; then
    echo "cache_keep_deps: no $dir, skipped"
    continue
  fi
  rm -rf "$dir/incremental" "$dir/bundle" "$dir/examples"
  # The profile's own outputs: the binaries and their build records.
  find "$dir" -maxdepth 1 -type f -delete
  # The test and binary executables; not the proc-macro libraries.
  find "$dir/deps" -maxdepth 1 -type f -perm -u+x \
    ! -name '*.so' ! -name '*.dylib' ! -name '*.dll' -delete 2>/dev/null || true
  find "$dir/deps" "$dir/.fingerprint" "$dir/build" -maxdepth 1 -name '*juggler*' -exec rm -rf {} + 2>/dev/null || true
done
rm -rf target/nextest target/tmp
du -sh target 2>/dev/null || true
