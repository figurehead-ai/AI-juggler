# shellcheck shell=bash
# AJ-E3822: the identifier each of AI Juggler's helpers is signed as, from
# the build's identity (D-23's editions; the app's own side is
# crates/juggler-app/src/edition.rs, Edition::helper_identifier). Sourced by
# the scripts that build, sign and check a helper; never run on its own.
#
# AI Juggler and AI Juggler Nightly share the team's signer, so their
# helpers share one identifier: ai.figurehead.juggler.<suffix>. Dev and lab
# builds are signed ad hoc, so theirs carry the Dev identity's prefix, as
# the app's own identifier does: ai.figurehead.juggler.dev.<suffix>. One
# identifier with two signers made macOS's app-data protection ask
# '"(null)" differs from previously opened versions' (0.5.0 Nightly).
#
#   helper_identifier SUFFIX
#       for this build's edition, AIJ_EDITION: main | nightly | dev
#       (unset: dev, so a build nobody named can never pass for a
#       release's; anything else stops the build).
#   helper_identifier_for_app APP SUFFIX
#       for the edition the .app's own identifier names (as the app
#       decides it, Edition::from_identifier: only the main and nightly
#       identifiers are a release's, anything else is Dev).
#   helper_plist TEMPLATE IDENTIFIER OUT
#       writes a helper's Info.plist from its template, its identifier in
#       place of @HELPER_IDENTIFIER@.

AIJ_RELEASE_PREFIX=ai.figurehead.juggler
AIJ_DEV_PREFIX=ai.figurehead.juggler.dev

helper_prefix() {
  case "${1:-dev}" in
    main | nightly) echo "$AIJ_RELEASE_PREFIX" ;;
    dev) echo "$AIJ_DEV_PREFIX" ;;
    *) echo "helper_identity: AIJ_EDITION is \"$1\"; it is main, nightly or dev" >&2; return 1 ;;
  esac
}

helper_identifier() {
  local prefix
  prefix="$(helper_prefix "${AIJ_EDITION:-dev}")" || return 1
  echo "$prefix.${1:?a helper suffix}"
}

helper_identifier_for_app() {
  local own
  own="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' "${1:?an app}/Contents/Info.plist")" || return 1
  case "$own" in
    "$AIJ_RELEASE_PREFIX" | "$AIJ_RELEASE_PREFIX.nightly") echo "$AIJ_RELEASE_PREFIX.${2:?a helper suffix}" ;;
    *) echo "$AIJ_DEV_PREFIX.${2:?a helper suffix}" ;;
  esac
}

helper_plist() {
  local template="$1" identifier="$2" out="$3"
  # Quoted into a code requirement and a plist: letters, digits, . and -.
  [[ "$identifier" =~ ^[A-Za-z0-9.-]+$ ]] || { echo "helper_identity: bad identifier \"$identifier\"" >&2; return 1; }
  grep -q '<string>@HELPER_IDENTIFIER@</string>' "$template" \
    || { echo "helper_identity: $template has no @HELPER_IDENTIFIER@" >&2; return 1; }
  sed "s|<string>@HELPER_IDENTIFIER@</string>|<string>$identifier</string>|" "$template" > "$out"
}

# The name every helper's embedded plist gives it, in any system prompt.
AIJ_HELPER_NAME="AI Juggler"

# Fails, saying why, unless BIN's embedded Info.plist carries IDENTIFIER
# and names it (CFBundleName and CFBundleDisplayName, AIJ_HELPER_NAME).
helper_plist_check() {
  local bin="$1" identifier="$2" dir status=0
  dir="$(mktemp -d)"
  if segedit "$bin" -extract __TEXT __info_plist "$dir/Info.plist" > /dev/null 2>&1; then
    python3 -I - "$dir/Info.plist" "$identifier" "$AIJ_HELPER_NAME" <<'PY' || status=1
import plistlib, sys
path, identifier, name = sys.argv[1:4]
with open(path, "rb") as f:
    plist = plistlib.load(f)
wrong = [f"{key} is {plist.get(key)!r}, not {want!r}" for key, want in (
    ("CFBundleIdentifier", identifier),
    ("CFBundleName", name),
    ("CFBundleDisplayName", name),
) if plist.get(key) != want]
if wrong:
    sys.exit("its embedded Info.plist: " + "; ".join(wrong))
PY
  else
    echo "it has no embedded Info.plist" >&2
    status=1
  fi
  rm -rf "$dir"
  return "$status"
}
