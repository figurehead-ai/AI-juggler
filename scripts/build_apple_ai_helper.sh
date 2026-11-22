#!/usr/bin/env bash
# ADR-0033 §4.1, AJ-E3802: AI Juggler's Apple helper, `juggler-apple-ai`,
# built with the system's Swift compiler, outside cargo, so the app stays
# Swift-free and `cargo run` never needs Swift. Without the helper the app
# says "helper missing" and every AI job keeps its plain path.
#
#   scripts/build_apple_ai_helper.sh [debug|release] [out-dir]
#       builds it into out-dir (default: the shared target's
#       $CARGO_TARGET_DIR/<profile>, where a development build looks),
#       signs it ad hoc, and checks it.
#   scripts/build_apple_ai_helper.sh sign <helper>
#       signs it again ad hoc, as the build does (after anything that
#       re-signed it without its entitlements: `codesign --deep`).
#   scripts/build_apple_ai_helper.sh check <helper> [<app>]
#       checks a built or bundled helper; with <app>, that it is signed by
#       the app's own team (the same TeamIdentifier, or none for both).
#
# What a helper must be (the app refuses anything else at run time,
# juggler_platform::code_trust): arm64 only (the system model needs Apple
# silicon); signed as ai.figurehead.juggler.apple-ai (its embedded
# Info.plist); hardened runtime; the App Sandbox and no other entitlement
# (helpers/juggler-apple-ai.entitlements); a valid signature; and no
# Private Cloud Compute symbol. The release signs it again with the
# app's Developer ID (scripts/sign_macos.sh), with the same entitlements.
# Deployment target macOS 14, the system model used only from macOS 26
# (weak-linked). No-op off macOS. Nothing is downloaded.
set -euo pipefail
cd "$(dirname "$0")/.."

[ "$(uname)" = Darwin ] || { echo "build_apple_ai_helper: macOS only; skipped"; exit 0; }

helpers=crates/juggler-app/helpers
identifier=ai.figurehead.juggler.apple-ai
entitlements="$helpers/juggler-apple-ai.entitlements"

fail() { echo "build_apple_ai_helper: $*" >&2; exit 1; }

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
  local info symbols want got
  info="$(codesign -dv "$bin" 2>&1)"
  grep -q '^CodeDirectory .*flags=.*runtime' <<< "$info" || fail "$bin: no hardened runtime"
  want="$(entries < "$entitlements")"
  got="$(codesign -d --entitlements - --xml "$bin" 2>/dev/null | entries)"
  [ "$got" = "$want" ] || fail "$bin's entitlements are $got, not $want"
  symbols="$(nm -a "$bin")"
  if grep -q PrivateCloudCompute <<< "$symbols"; then
    fail "$bin names Private Cloud Compute"
  fi
  if [ -n "$app" ]; then
    [ "$(team_of "$bin")" = "$(team_of "$app")" ] \
      || fail "$bin's team ($(team_of "$bin")) isn't the app's ($(team_of "$app"))"
  fi
  echo "build_apple_ai_helper: $bin checked (arm64, $identifier, runtime, sandbox, team $(team_of "$bin"))"
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
out="${2:-$target/$profile}"
mkdir -p "$out"

swiftc -O -parse-as-library \
  -target arm64-apple-macos14.0 \
  -Xlinker -weak_framework -Xlinker FoundationModels \
  -Xlinker -sectcreate -Xlinker __TEXT -Xlinker __info_plist -Xlinker "$helpers/juggler-apple-ai.Info.plist" \
  "$helpers/juggler-apple-ai.swift" \
  -o "$out/juggler-apple-ai"
chmod 0755 "$out/juggler-apple-ai"
sign_ad_hoc "$out/juggler-apple-ai"
check "$out/juggler-apple-ai"
