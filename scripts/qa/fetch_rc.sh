#!/usr/bin/env bash
# A release candidate from CI, ready to walk (2026-11-11: release
# candidates are built on GitHub, not on this Mac). CI's `rc-dev-app` job
# builds AI Juggler Dev for a commit, checks it (bundle, first open after
# an update) and uploads it as `rc-dev-app-<commit>`; this fetches it:
#
#   scripts/qa/fetch_rc.sh <commit>
#
# - downloads the artifact (`gh run download`) of a CI run of that commit
#   (the pull request's head; the newest run that has it);
# - unpacks the .app into <rc folder>/rc-<sha12>/ (default: the
#   AIJuggler-shared-target/rc folder beside the main checkout; AIJ_RC_DIR
#   overrides), with a COMMIT file;
# - removes the quarantine attribute, so Gatekeeper doesn't block or
#   translocate the unsigned app;
# - makes the folders owner-only (700): juggler-agent's trust check refuses
#   a helper with an ancestor others can write (an app under /private/tmp
#   got no state reports and no pool tools, the rc6 walk);
# - prints the launch line.
#
# What differs from a .app built on this Mac: docs/RELEASING.md,
# "Release candidates".
set -euo pipefail

want=${1:?usage: fetch_rc.sh <commit>}
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"
die() { echo "fetch-rc: $*" >&2; exit 1; }

full="$(git rev-parse -q --verify "${want}^{commit}" 2>/dev/null || true)"
if [ -z "$full" ]; then
  git fetch -q origin 2>/dev/null || true
  full="$(git rev-parse -q --verify "${want}^{commit}" 2>/dev/null || true)"
fi
[ -n "$full" ] || die "no commit $want here (fetch it first, or pass the full SHA)"

COMMON="$(cd "$(git rev-parse --git-common-dir)" && pwd)"
RC_ROOT="${AIJ_RC_DIR:-$(dirname "$(dirname "$COMMON")")/AIJuggler-shared-target/rc}"
dest="$RC_ROOT/rc-${full:0:12}"
app="$dest/AI Juggler Dev.app"
name="rc-dev-app-$full"

# Never replace an app that is running.
if pgrep -f "$app/Contents/MacOS/juggler-app" >/dev/null 2>&1; then
  die "$app is running; quit it first"
fi

mkdir -p "$RC_ROOT"
chmod 700 "$RC_ROOT"
tmp="$(mktemp -d "$RC_ROOT/.fetch.XXXXXX")"
trap 'rm -rf "$tmp"' EXIT

runs="$(gh run list --workflow ci.yml --commit "$full" --limit 20 --json databaseId --jq '.[].databaseId')"
[ -n "$runs" ] || die "no CI run for ${full:0:12} yet (pushed? its pull request open?)"
got=""
for id in $runs; do
  if gh run download "$id" --name "$name" --dir "$tmp/artifact" >/dev/null 2>&1; then
    got=$id
    break
  fi
  rm -rf "$tmp/artifact"
done
if [ -z "$got" ]; then
  echo "fetch-rc: no $name artifact yet; the runs of ${full:0:12}:" >&2
  gh run list --workflow ci.yml --commit "$full" --limit 5 >&2 || true
  die "wait for its rc-dev-app job (or see why it failed: gh run view <id>)"
fi
[ -f "$tmp/artifact/app.tar" ] || die "the artifact of run $got has no app.tar"
[ "$(cat "$tmp/artifact/COMMIT" 2>/dev/null)" = "$full" ] || die "run $got's artifact names another commit"

rm -rf "$dest"
mkdir -p "$dest"
tar -xf "$tmp/artifact/app.tar" -C "$dest"
[ -d "$app" ] || die "the artifact has no AI Juggler Dev.app"
echo "$full" > "$dest/COMMIT"
xattr -r -d com.apple.quarantine "$dest" 2>/dev/null || true
if xattr -r -l "$dest" 2>/dev/null | grep -q com.apple.quarantine; then
  die "couldn't remove the quarantine attribute from $dest"
fi
chmod 700 "$dest"
python3 scripts/bundle_check.py "$app"

# The helper's trust rule (juggler-platform file_trust): every folder above
# it root's or ours, and writable by no one else (a group only when it is
# the administrators').
dir="$dest"
while [ "$dir" != / ]; do
  read -r mode owner group <<<"$(stat -f '%Lp %u %g' "$dir")"
  if { [ "$owner" != 0 ] && [ "$owner" != "$(id -u)" ]; } \
    || [ $((8#$mode & 8#002)) -ne 0 ] \
    || { [ $((8#$mode & 8#020)) -ne 0 ] && [ "$group" != 0 ] && [ "$group" != 80 ]; }; then
    die "$dir is writable by others: juggler-agent would refuse to run from under it (set AIJ_RC_DIR elsewhere)"
  fi
  dir="$(dirname "$dir")"
done

echo "fetch-rc: ${full:0:12} from CI run $got at $dest"
echo "launch with: open -n --env AIJ_NO_RESTORE=1 \"$app\""
