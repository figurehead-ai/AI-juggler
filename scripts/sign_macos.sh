#!/usr/bin/env bash
# AJ-E2001: sign, notarize and package one edition of an unsigned build —
# the only place AI Juggler's release keys are used. It runs in the
# `release` environment's sign job, which builds nothing: it runs Apple's
# tools, the pinned prebuilt tauri-cli (`signer sign`) and this
# repository's scripts, never a dependency's code.
#
#   sign_macos.sh UNSIGNED_TAR OUT_DIR
#
# UNSIGNED_TAR: the build job's `app.tar` (one unsigned `<Product>.app`).
# OUT_DIR gets `<PREFIX>-<VERSION>-universal.dmg`, `.app.tar.gz` and
# `.app.tar.gz.sig`.
#
# Environment: VERSION, PREFIX, AIJ_UPDATER_PUBKEY, and either
#   REHEARSAL=1 — ad-hoc signature, no notarization, a throwaway updater
#     key (the dry run on a branch: no secret is needed or read); or
#   APPLE_CERTIFICATE (.p12, base64), APPLE_CERTIFICATE_PASSWORD,
#   APPLE_SIGNING_IDENTITY, APPLE_TEAM_ID, TAURI_SIGNING_PRIVATE_KEY,
#   TAURI_SIGNING_PRIVATE_KEY_PASSWORD, and notarization credentials: an
#   App Store Connect API key (APPLE_API_KEY = the .p8's text,
#   APPLE_API_KEY_ID, APPLE_API_ISSUER; preferred) or APPLE_ID and
#   APPLE_PASSWORD (an app-specific password).
#
# Each credential lives in a file under a private temporary folder only
# while it is used, and the temporary keychain is deleted on exit.
set -euo pipefail
# What users install is world-readable; credentials get 077 where written.
umask 022

tarball="$1"
out="$2"
here="$(cd "$(dirname "$0")" && pwd)"
retry=(python3 "$here/apple_retry.py" --)
# hdiutil again while the image is busy (exit 16, EBUSY): macOS's disk
# image helper or Spotlight can still hold an image just made or
# detached, as on the 2026-10-21 rehearsal's Nightly edition. Any other
# failure stops at once.
hdi() {
  local attempt status
  for attempt in 1 2 3 4 5; do
    status=0
    hdiutil "$@" || status=$?
    [ "$status" -eq 16 ] || return "$status"
    echo "hdiutil $1: the image is busy (try $attempt of 5)" >&2
    sleep $((attempt * 3))
  done
  return 16
}
# Detach a mounted image. A busy detach can still finish on its own a
# moment later, so the next try then fails with "not mounted" (the 0.2.0
# rehearsal's Nightly edition): once the mount point is gone, it's done.
# The last try forces it.
detach() {
  local mount attempt status
  mount="$(cd "$1" && pwd -P)"
  for attempt in 1 2 3 4 5; do
    status=0
    if [ "$attempt" -lt 5 ]; then
      hdiutil detach -quiet "$mount" || status=$?
    else
      hdiutil detach -quiet -force "$mount" || status=$?
    fi
    [ "$status" -eq 0 ] && return 0
    mounted "$mount" || return 0
    echo "hdiutil detach: the image is busy (try $attempt of 5, exit $status)" >&2
    sleep $((attempt * 3))
  done
  return "$status"
}
mounted() {
  mount | grep -qF " on $1 ("
}
rehearsal="${REHEARSAL:-}"
: "${VERSION:?}" "${PREFIX:?}"

work="$(mktemp -d "${RUNNER_TEMP:-${TMPDIR:-/tmp}}/sign.XXXXXX")"
chmod 700 "$work"
keychain=""
cleanup() {
  if [ -n "$keychain" ]; then security delete-keychain "$keychain" 2>/dev/null || true; fi
  rm -rf -- "$work"
}
trap cleanup EXIT

mkdir -p "$out" "$work/app"
tar -xpf "$tarball" -C "$work/app"
apps=("$work"/app/*.app)
[ ${#apps[@]} -eq 1 ] && [ -d "${apps[0]}" ] || { echo "::error::expected one .app in $tarball"; exit 1; }
app="${apps[0]}"
product="$(basename "$app" .app)"
# Everyone may read and run what is installed; nobody else may write it
# (whatever umask the build ran under). Not part of the signature.
chmod -R u+rwX,go+rX,go-w "$app"

# Nothing is signed with --deep: every piece of code is named here and
# signed on its own. The bundle holds the main executable and the helpers
# listed in `helpers` (AJ-E1704: the agent bridge, `juggler-agent`), each
# in Contents/MacOS; anything else that could carry code stops here.
helpers=(juggler-agent)
main="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleExecutable' "$app/Contents/Info.plist")"
extra="$(find "$app/Contents" -mindepth 1 -maxdepth 1 ! -name MacOS ! -name Resources ! -name Info.plist)"
unexpected=""
while IFS= read -r file; do
  name="$(basename "$file")"
  [ "$name" = "$main" ] && continue
  case " ${helpers[*]} " in
    *" $name "*) ;;
    *) unexpected="$unexpected $name" ;;
  esac
done < <(find "$app/Contents/MacOS" -type f)
if [ -n "$extra" ] || [ -n "$unexpected" ] || [ ! -f "$app/Contents/MacOS/$main" ]; then
  echo "::error::the bundle has code beyond its executable and its named helpers; sign it explicitly first:"
  find "$app/Contents" -maxdepth 2
  exit 1
fi
for helper in "${helpers[@]}"; do
  [ -f "$app/Contents/MacOS/$helper" ] || { echo "::error::the bundle lacks its helper $helper"; exit 1; }
done

# --- credentials -----------------------------------------------------------
if [ -n "$rehearsal" ]; then
  identity="-"
  # A throwaway updater key, made by the same pinned tauri-cli.
  cargo-tauri signer generate --ci -p rehearsal -w "$work/updater.key" -f >/dev/null
  updater_key="$(cat "$work/updater.key")"
  updater_password=rehearsal
  updater_pubkey="$(cat "$work/updater.key.pub")"
else
  : "${APPLE_CERTIFICATE:?}" "${APPLE_CERTIFICATE_PASSWORD:?}" "${APPLE_SIGNING_IDENTITY:?}" \
    "${APPLE_TEAM_ID:?}" "${TAURI_SIGNING_PRIVATE_KEY:?}" "${TAURI_SIGNING_PRIVATE_KEY_PASSWORD:?}" \
    "${AIJ_UPDATER_PUBKEY:?}"
  identity="$APPLE_SIGNING_IDENTITY"
  updater_key="$TAURI_SIGNING_PRIVATE_KEY"
  updater_password="$TAURI_SIGNING_PRIVATE_KEY_PASSWORD"
  updater_pubkey="$AIJ_UPDATER_PUBKEY"

  keychain="$work/signing.keychain-db"
  keychain_password="$(openssl rand -hex 24)"
  # A derived secret GitHub doesn't know: masked in case anything prints it.
  [ -n "${GITHUB_ACTIONS:-}" ] && echo "::add-mask::$keychain_password"
  security create-keychain -p "$keychain_password" "$keychain"
  security set-keychain-settings -lut 21600 "$keychain"
  security unlock-keychain -p "$keychain_password" "$keychain"
  ( umask 077 && printf '%s' "$APPLE_CERTIFICATE" | base64 --decode > "$work/cert.p12" )
  security import "$work/cert.p12" -k "$keychain" -P "$APPLE_CERTIFICATE_PASSWORD" -T /usr/bin/codesign >/dev/null
  rm -f -- "$work/cert.p12"
  security set-key-partition-list -S apple-tool:,apple:,codesign: -s -k "$keychain_password" "$keychain" >/dev/null
  # codesign looks the identity up in the search list.
  existing=()
  while IFS= read -r line; do existing+=("$(echo "$line" | tr -d ' "')"); done < <(security list-keychains -d user)
  security list-keychains -d user -s "$keychain" "${existing[@]}"

  if [ -n "${APPLE_API_KEY:-}" ]; then
    : "${APPLE_API_KEY_ID:?}" "${APPLE_API_ISSUER:?}"
    ( umask 077 && printf '%s' "$APPLE_API_KEY" > "$work/notary.p8" )
    notary=(--key "$work/notary.p8" --key-id "$APPLE_API_KEY_ID" --issuer "$APPLE_API_ISSUER")
  else
    : "${APPLE_ID:?}" "${APPLE_PASSWORD:?}"
    notary=(--apple-id "$APPLE_ID" --password "$APPLE_PASSWORD" --team-id "$APPLE_TEAM_ID")
  fi
fi

sign() {
  if [ -n "$rehearsal" ]; then
    codesign --force --options runtime --sign - "$@"
  else
    "${retry[@]}" codesign --force --options runtime --timestamp --keychain "$keychain" --sign "$identity" "$@"
  fi
}

notarize() {
  [ -n "$rehearsal" ] && { echo "rehearsal: not notarizing $(basename "$1")"; return; }
  "${retry[@]}" xcrun notarytool submit "$1" "${notary[@]}" --wait --timeout 90m
}

staple() {
  [ -n "$rehearsal" ] && return
  # A fresh ticket can take a moment to reach Apple's CDN.
  "${retry[@]}" xcrun stapler staple "$1"
}

# --- the app ---------------------------------------------------------------
# Inside out: each helper first (hardened runtime, no entitlements), then
# the app, whose signature seals them.
for helper in "${helpers[@]}"; do
  sign "$app/Contents/MacOS/$helper"
done
sign "$app"
codesign --verify --strict --verbose=2 "$app"
for helper in "${helpers[@]}"; do
  codesign --verify --strict --verbose=2 "$app/Contents/MacOS/$helper"
  if [ -n "$(codesign -d --entitlements - --xml "$app/Contents/MacOS/$helper" 2>/dev/null || true)" ]; then
    echo "::error::the helper $helper carries entitlements"
    exit 1
  fi
done
# Release builds carry no entitlements: no get-task-allow and no hardened
# runtime exception (SECURITY-GUIDELINES C8).
entitlements="$(codesign -d --entitlements - --xml "$app" 2>/dev/null || true)"
if printf '%s' "$entitlements" | grep -Eq 'get-task-allow|com\.apple\.security\.cs\.'; then
  echo "::error::the app carries a debugging or runtime-exception entitlement"
  exit 1
fi
if [ -z "$rehearsal" ]; then
  for code in "$app" "${helpers[@]/#/$app/Contents/MacOS/}"; do
    codesign -dvv "$code" 2>&1 | grep -q 'flags=.*runtime' || { echo "::error::no hardened runtime: $code"; exit 1; }
  done
fi
ditto -c -k --keepParent "$app" "$work/notarize.zip"
notarize "$work/notarize.zip"
staple "$app"

# --- the updater archive (as tauri-bundler makes it: <Product>.app at the
# root, symlinks kept), from the stapled app, signed with the updater key.
archive="$out/${PREFIX}-${VERSION}-universal.app.tar.gz"
( cd "$(dirname "$app")" && COPYFILE_DISABLE=1 tar --no-mac-metadata --no-xattrs -czf "$archive" "$product.app" )
# --app-version binds the signature to this version: the app's updater
# requires it (requireSignedVersion), as `tauri build` used to add it.
TAURI_SIGNING_PRIVATE_KEY="$updater_key" TAURI_SIGNING_PRIVATE_KEY_PASSWORD="$updater_password" \
  cargo-tauri signer sign --app-version "$VERSION" "$archive" >/dev/null
python3 "$here/updater_sig_check.py" "$archive" "$archive.sig" "$updater_pubkey" --version "$VERSION"

# --- the disk image: the app, a link to Applications, the volume icon -----
stage="$work/dmg"
mkdir -p "$stage"
ditto "$app" "$stage/$product.app"
ln -s /Applications "$stage/Applications"
hdi create -quiet -volname "$product" -srcfolder "$stage" -fs HFS+ -format UDRW -ov "$work/rw.dmg"
icon="$(find "$app/Contents/Resources" -name '*.icns' | head -1)"
if [ -n "$icon" ] && [ -x /usr/bin/SetFile ]; then
  mount="$work/mount"
  mkdir -p "$mount"
  hdi attach -quiet -nobrowse -mountpoint "$mount" "$work/rw.dmg"
  cp "$icon" "$mount/.VolumeIcon.icns"
  chmod 644 "$mount/.VolumeIcon.icns"
  /usr/bin/SetFile -a C "$mount"
  detach "$mount"
fi
dmg="$out/${PREFIX}-${VERSION}-universal.dmg"
hdi convert -quiet "$work/rw.dmg" -format UDZO -o "$dmg" -ov
sign "$dmg"
notarize "$dmg"
staple "$dmg"

# --- verify what users will get ----------------------------------------------
codesign --verify --deep --strict --verbose=2 "$app"
# No test seam ships (SECURITY-GUIDELINES C8): the variables that silence
# or answer dialogs, or start the crash watchdog in a debug build, are
# compiled into debug builds only.
"$here/release_seams_check.sh" "$app"/Contents/MacOS/*
if [ -z "$rehearsal" ]; then
  "${retry[@]}" spctl --assess --type execute --verbose=4 "$app"
  xcrun stapler validate "$app"
  xcrun stapler validate "$dmg"
  "${retry[@]}" spctl --assess --type open --context context:primary-signature --verbose=4 "$dmg"
fi
ls -l "$out"
