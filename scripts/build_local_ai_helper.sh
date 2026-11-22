#!/usr/bin/env bash
# ADR-0033 §4.2, D-131: AI Juggler's local-model helper, `juggler-local-ai`
# (llama.cpp through llama-cpp-2 0.1.146, Metal), built from its own
# workspace (crates/juggler-local-ai, outside the app's, so `just verify`
# never compiles llama.cpp's C++). Without it the app says "helper
# missing" for downloaded models and every AI job keeps its other paths.
#
#   scripts/build_local_ai_helper.sh [debug|release] [out-dir]
#       builds it OFFLINE and LOCKED from the cargo cache (--offline
#       --locked: a missing or different crate fails, nothing is fetched;
#       a release's version stamp updates the protocol crate's entry in the
#       helper's lock, scripts/release_version.py), with `cargo auditable`
#       where it is installed (the release's compile jobs), into out-dir
#       (default:
#       $CARGO_TARGET_DIR/<profile>, where a development build looks),
#       signs it ad hoc, and checks it.
#   scripts/build_local_ai_helper.sh sign <helper>
#       signs it again ad hoc, as the build does (after anything that
#       re-signed it without its hardened runtime: `codesign --deep`).
#   scripts/build_local_ai_helper.sh check <helper> [<app>]
#       checks a built or bundled helper; with <app>, that it is signed by
#       the app's own team (the same TeamIdentifier, or none for both).
#
# What a helper must be (the app refuses anything else at run time,
# juggler_platform::code_trust): arm64 only (Apple silicon, ADR-0033 §4.2);
# signed as ai.figurehead.juggler.local-ai (its embedded Info.plist);
# hardened runtime; NO entitlements (its sandbox isn't the App Sandbox but
# its own deny-by-default profile, juggler_local_ai_protocol::SANDBOX_PROFILE,
# which the app applies with /usr/bin/sandbox-exec at every start: the
# App Sandbox can't deny /private/tmp reads or LaunchServices, and the two
# can't be combined); a valid signature; and no networking call imported.
# The import checks are hygiene, not the boundary: the sandbox is (it
# denies the network, every file outside the models' folder and the system
# libraries, and fork/exec; crates/juggler-local-ai/tests/protocol.rs
# probes each). ggml's crash handler imports fork and execlp to start a
# debugger only when GGML_BACKTRACE_LLDB is set; the app's cleared
# environment never sets it (and sets GGML_NO_BACKTRACE=1), and the
# sandbox denies the exec. The release signs it again with the app's
# Developer ID (scripts/sign_macos.sh). No-op off macOS. Nothing is
# downloaded.
set -euo pipefail
cd "$(dirname "$0")/.."

[ "$(uname)" = Darwin ] || { echo "build_local_ai_helper: macOS only; skipped"; exit 0; }

crate=crates/juggler-local-ai
identifier=ai.figurehead.juggler.local-ai

fail() { echo "build_local_ai_helper: $*" >&2; exit 1; }

team_of() {
  codesign -dv "$1" 2>&1 | sed -n 's/^TeamIdentifier=//p'
}

check() {
  local bin="$1" app="${2:-}"
  [ -f "$bin" ] && [ -x "$bin" ] || fail "no helper at $bin"
  [ "$(lipo -archs "$bin")" = arm64 ] || fail "$bin isn't arm64 only: $(lipo -archs "$bin")"
  codesign --verify --strict "$bin" || fail "$bin: invalid signature"
  codesign --verify --strict -R="identifier \"$identifier\"" "$bin" \
    || fail "$bin isn't signed as $identifier"
  # Each tool's whole output first: `grep -q` stops reading early, and
  # under pipefail the writer's SIGPIPE would turn a match into a miss.
  local info imports got
  info="$(codesign -dv "$bin" 2>&1)"
  grep -q '^CodeDirectory .*flags=.*runtime' <<< "$info" || fail "$bin: no hardened runtime"
  got="$(codesign -d --entitlements - --xml "$bin" 2>/dev/null || true)"
  [ -z "$got" ] || fail "$bin carries entitlements (its sandbox is its own profile, applied at start)"
  # No network: none of the socket calls is even imported (llama.cpp's
  # download code isn't built: CMAKE_DISABLE_FIND_PACKAGE_OpenSSL, no curl).
  imports="$(nm -u "$bin")"
  if grep -qE '^_(socket|connect|getaddrinfo|bind|listen|accept|sendto|recvfrom|curl_.*|SSL_.*)$' <<< "$imports"; then
    fail "$bin imports a network call: $(grep -E '^_(socket|connect|getaddrinfo|bind|listen|accept|sendto|recvfrom|curl_.*|SSL_.*)$' <<< "$imports" | tr '\n' ' ')"
  fi
  # Starting another program: only ggml's crash backtrace (fork + execlp
  # of a debugger, only with GGML_BACKTRACE_LLDB set, which the app's
  # cleared environment never is; the sandbox denies it anyway); any other
  # process call imported is refused.
  local spawns
  spawns="$(grep -E '^_(fork|vfork|execl|execlp|execle|execv|execvp|execvP|execve|posix_spawn|posix_spawnp|system|popen)$' <<< "$imports" | tr '\n' ' ' || true)"
  [ "$spawns" = "_execlp _fork " ] || [ -z "$spawns" ] || fail "$bin imports process calls: $spawns"
  if [ -n "$app" ]; then
    [ "$(team_of "$bin")" = "$(team_of "$app")" ] \
      || fail "$bin's team ($(team_of "$bin")) isn't the app's ($(team_of "$app"))"
  fi
  echo "build_local_ai_helper: $bin checked (arm64, $identifier, runtime, no entitlements, no network imports, team $(team_of "$bin"))"
}

# Ad hoc; the release signs it again with the Developer ID.
sign_ad_hoc() {
  codesign --force --options runtime --sign - "$1" 2>/dev/null
}

case "${1:-}" in
  check) check "${2:?a helper}" "${3:-}"; exit 0 ;;
  sign) sign_ad_hoc "${2:?a helper}"; check "$2"; exit 0 ;;
esac

profile="${1:-debug}"
target="${CARGO_TARGET_DIR:-target}"
case "$target" in /*) ;; *) target="$PWD/$target" ;; esac
out="${2:-$target/$profile}"
case "$out" in /*) ;; *) out="$PWD/$out" ;; esac
mkdir -p "$out"

# The helper is always an optimised build (llama.cpp is unusable without),
# for arm64, from the cargo cache only. CMAKE_DISABLE_FIND_PACKAGE_OpenSSL
# keeps llama.cpp's model-download code (cpp-httplib) out of the build.
build=(cargo build)
if cargo auditable --version > /dev/null 2>&1; then build=(cargo auditable build); fi
(
  cd "$crate"
  CMAKE_DISABLE_FIND_PACKAGE_OpenSSL=TRUE CARGO_TARGET_DIR="$target" \
    "${build[@]}" --offline --locked --release --target aarch64-apple-darwin
)
built="$target/aarch64-apple-darwin/release/juggler-local-ai"
[ -f "$built" ] || fail "cargo built no helper at $built"
# The release workflow's out-dir on arm64 (target/aarch64-apple-darwin/
# release) is cargo's own output folder: the built file is already there.
# Removing "the old copy" first deleted the build itself (0.5's release
# PR, rehearse aarch64: "cp: … No such file or directory").
if [ "$(cd "$(dirname "$built")" && pwd -P)" != "$(cd "$out" && pwd -P)" ]; then
  rm -f "$out/juggler-local-ai"
  cp "$built" "$out/juggler-local-ai"
fi
chmod 0755 "$out/juggler-local-ai"
sign_ad_hoc "$out/juggler-local-ai"
check "$out/juggler-local-ai"
