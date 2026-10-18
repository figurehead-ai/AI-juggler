#!/usr/bin/env bash
# CLT-only macOS dev machines: the screencapturekit v11 Swift bridge
# (AJ-E415) references `@rpath/libswift_Concurrency.dylib`, which ships
# with FULL Xcode toolchains only. CI is unaffected (Linux verify never
# links it; build-macos compiles but never runs the binary). This
# script makes local `cargo test/run` work without Xcode by (a) linking
# the CLT static compat lib and (b) providing a link-time stub for the
# dylib load command — at RUNTIME the OS's shared-cache copy under
# /usr/lib/swift wins (rpath order below), supplying the real Swift
# Concurrency symbols the bridge calls.
# No-op on non-macOS and when a full Xcode is selected.
set -euo pipefail

if [ "$(uname)" != "Darwin" ]; then
    exit 0
fi
if xcode-select -p 2>/dev/null | grep -q "Xcode.app"; then
    exit 0
fi

CLT_SWIFT="/Library/Developer/CommandLineTools/usr/lib/swift/macosx"
if [ ! -f "$CLT_SWIFT/libswiftCompatibility56.a" ]; then
    echo "ensure_swift_runtime: CLT swift libs missing at $CLT_SWIFT" >&2
    exit 0
fi

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
mkdir -p "$REPO_ROOT/target/debug"
STUB="$REPO_ROOT/target/debug/libswift_Concurrency.dylib"
if [ ! -f "$STUB" ]; then
    # An empty dylib with the expected install name: the bridge loads
    # it but binds none of its symbols.
    echo "" | clang -dynamiclib -o "$STUB" \
        -install_name @rpath/libswift_Concurrency.dylib -x c -
fi

# rpath ORDER is load-bearing (AJ-E415 real-capture fix): /usr/lib/swift
# comes FIRST — the OS provides libswift_Concurrency.dylib from the dyld
# shared cache even without a file on disk (verified: dlopen +
# swift_task_create resolve). The empty stub in target/debug stays as a
# LINK-time satisfier only; at runtime the OS runtime wins, so the
# bridge's _swift_task_* calls land on real symbols instead of NULL.
# Exported by callers via $(scripts/ensure_swift_runtime.sh).
echo "RUSTFLAGS=-C link-arg=-L$CLT_SWIFT -C link-arg=-lswiftCompatibility56 -C link-arg=-Wl,-rpath,/usr/lib/swift -C link-arg=-Wl,-rpath,$REPO_ROOT/target/debug"
