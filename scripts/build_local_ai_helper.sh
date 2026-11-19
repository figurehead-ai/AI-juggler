#!/usr/bin/env bash
# ADR-0033 §4.2, D-131: AI Juggler's local-model helper, `juggler-local-ai`
# (llama.cpp through llama-cpp-2 0.1.146, Metal), built from its own
# workspace (crates/juggler-local-ai, outside the app's, so `just verify`
# never compiles llama.cpp's C++). Without it the app says "helper
# missing" for downloaded models and every AI job keeps its other paths.
#
#   scripts/build_local_ai_helper.sh [debug|release] [out-dir]
#       builds it OFFLINE from the cargo cache (--offline: a missing crate
#       fails, nothing is fetched; not --locked, because a release's
#       version stamp changes the protocol crate's version in the lock;
#       llama-cpp-2 is pinned with `=` and CI's own job builds --locked)
#       into out-dir (default:
#       $CARGO_TARGET_DIR/<profile>, where a development build looks),
#       signs it ad hoc, and checks it.
#   scripts/build_local_ai_helper.sh sign <helper>
#       signs it again ad hoc, as the build does (after anything that
#       re-signed it without its entitlements: `codesign --deep`).
#   scripts/build_local_ai_helper.sh check <helper> [<app>]
#       checks a built or bundled helper; with <app>, that it is signed by
#       the app's own team (the same TeamIdentifier, or none for both).
#
# What a helper must be (the app refuses anything else at run time,
# juggler_platform::code_trust): arm64 only (Apple silicon, ADR-0033 §4.2);
# signed as ai.figurehead.juggler.local-ai (its embedded Info.plist);
# hardened runtime; exactly its entitlements (the App Sandbox, no network,
# read-only access to the models' folders:
# crates/juggler-local-ai/juggler-local-ai.entitlements); a valid
# signature; and no networking or process-starting call it could make
# outside llama.cpp's crash backtrace (switched off by the app's
# GGML_NO_BACKTRACE=1, and refused by the sandbox anyway). The release
# signs it again with the app's Developer ID (scripts/sign_macos.sh), with
# the same entitlements. No-op off macOS. Nothing is downloaded.
set -euo pipefail
cd "$(dirname "$0")/.."

[ "$(uname)" = Darwin ] || { echo "build_local_ai_helper: macOS only; skipped"; exit 0; }

crate=crates/juggler-local-ai
identifier=ai.figurehead.juggler.local-ai
entitlements="$crate/juggler-local-ai.entitlements"

fail() { echo "build_local_ai_helper: $*" >&2; exit 1; }

# A plist's entries, sorted, for comparing entitlements exactly.
entries() {
  python3 -I -c 'import plistlib, sys
data = sys.stdin.buffer.read()
print(sorted(plistlib.loads(data).items()) if data.strip() else [])'
}

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
  local info imports want got
  info="$(codesign -dv "$bin" 2>&1)"
  grep -q '^CodeDirectory .*flags=.*runtime' <<< "$info" || fail "$bin: no hardened runtime"
  want="$(entries < "$entitlements")"
  got="$(codesign -d --entitlements - --xml "$bin" 2>/dev/null | entries)"
  [ "$got" = "$want" ] || fail "$bin's entitlements are $got, not $want"
  # No network: none of the socket calls is even imported (llama.cpp's
  # download code isn't built: CMAKE_DISABLE_FIND_PACKAGE_OpenSSL, no curl).
  imports="$(nm -u "$bin")"
  if grep -qE '^_(socket|connect|getaddrinfo|bind|listen|accept|sendto|recvfrom|curl_.*|SSL_.*)$' <<< "$imports"; then
    fail "$bin imports a network call: $(grep -E '^_(socket|connect|getaddrinfo|bind|listen|accept|sendto|recvfrom|curl_.*|SSL_.*)$' <<< "$imports" | tr '\n' ' ')"
  fi
  # Starting another program: only ggml's crash backtrace (fork + execlp
  # of a debugger), which the app turns off; anything more is refused.
  local spawns
  spawns="$(grep -E '^_(fork|vfork|execl|execlp|execle|execv|execvp|execvP|execve|posix_spawn|posix_spawnp|system|popen)$' <<< "$imports" | tr '\n' ' ' || true)"
  [ "$spawns" = "_execlp _fork " ] || [ -z "$spawns" ] || fail "$bin imports process calls: $spawns"
  if [ -n "$app" ]; then
    [ "$(team_of "$bin")" = "$(team_of "$app")" ] \
      || fail "$bin's team ($(team_of "$bin")) isn't the app's ($(team_of "$app"))"
  fi
  echo "build_local_ai_helper: $bin checked (arm64, $identifier, runtime, sandbox, no network imports, team $(team_of "$bin"))"
}

# Ad hoc; the release signs it again with the Developer ID.
sign_ad_hoc() {
  codesign --force --options runtime --entitlements "$entitlements" --sign - "$1" 2>/dev/null
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
(
  cd "$crate"
  CMAKE_DISABLE_FIND_PACKAGE_OpenSSL=TRUE CARGO_TARGET_DIR="$target" \
    cargo build --offline --release --target aarch64-apple-darwin
)
built="$target/aarch64-apple-darwin/release/juggler-local-ai"
rm -f "$out/juggler-local-ai"
cp "$built" "$out/juggler-local-ai"
chmod 0755 "$out/juggler-local-ai"
sign_ad_hoc "$out/juggler-local-ai"
check "$out/juggler-local-ai"
