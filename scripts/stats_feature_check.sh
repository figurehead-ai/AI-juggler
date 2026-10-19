#!/usr/bin/env bash
# AJ-E1102: the `stats` cargo feature decides whether usage statistics are
# in the app at all. With it (the default) juggler-stats is a dependency;
# without it (--no-default-features) the crate is not in the build.
set -euo pipefail
tree() { cargo tree -p juggler-app -e normal --prefix none "$@" 2>/dev/null; }
if ! tree | grep -q '^juggler-stats '; then
  echo "stats-feature: juggler-stats missing from the default build" >&2
  exit 1
fi
if tree --no-default-features | grep -q '^juggler-stats '; then
  echo "stats-feature: juggler-stats is in a build without the stats feature" >&2
  exit 1
fi
echo "stats-feature: in the default build, absent without the feature"
