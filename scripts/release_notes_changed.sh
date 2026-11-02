#!/usr/bin/env bash
# The release notes this branch changed, checked as the nightly and
# promote check them (scripts/release_notes_check.sh): 0.3.2's nightly
# stopped on its notes after the merge. Run by preflight and CI's static
# job. Usage: release_notes_changed.sh BASE   (a ref, e.g. origin/main)
set -euo pipefail
base="${1:?usage: release_notes_changed.sh BASE}"
cd "$(dirname "$0")/.."
from="$(git merge-base "$base" HEAD 2>/dev/null || echo "$base")"
checked=0
for file in $(git diff --name-only "$from" HEAD -- 'docs/releases/*.md'); do
  [ -f "$file" ] || continue # removed
  version="$(basename "$file" .md)"
  [[ $version =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || continue
  ./scripts/release_notes_check.sh "$version"
  checked=$((checked + 1))
done
echo "release-notes: $checked changed version(s) checked against $base"
