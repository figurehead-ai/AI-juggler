#!/usr/bin/env bash
# The version a release candidate will ship as, stamped into the build
# (semver-strict: every build shows the version it ships as; the 0.4.2 RC
# showed "v0.4.0", the workspace's unstamped version). A release branch
# (release/X.Y.Z) ships as X.Y.Z; any other build as the next release
# after the tags (release_version.py next). Prints the version, and
# writes AIJ_EXPECT_VERSION for bundle_check when GITHUB_ENV is set.
#
#   HEAD_REF=<branch> scripts/ci/rc_version.sh
set -euo pipefail
cd "$(dirname "$0")/../.."
ref="${HEAD_REF:-}"
case "$ref" in
  release/[0-9]*.[0-9]*.[0-9]*) version="${ref#release/}" ;;
  *) version="$(git ls-remote --tags --refs origin 'v*' | sed 's#.*refs/tags/##' | python3 scripts/release_version.py next)" ;;
esac
python3 scripts/release_version.py stamp "$version"
if [ -n "${GITHUB_ENV:-}" ]; then echo "AIJ_EXPECT_VERSION=$version" >> "$GITHUB_ENV"; fi
echo "$version"
