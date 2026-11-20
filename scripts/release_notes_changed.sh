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
  # Another release's notes that a forward merge brought in unchanged
  # (0.4.7's into release/0.5) are checked by that release against its
  # own tree; checked here they'd be measured against this branch's
  # schema and code (0.5's preflight: "0.4.7 needs a downgrade note" for
  # 0.5's own V55).
  if git cat-file -e "origin/main:$file" 2>/dev/null && git diff --quiet origin/main HEAD -- "$file"; then
    echo "release-notes: $file is main's, unchanged here (checked by its own release)"
    continue
  fi
  ./scripts/release_notes_check.sh "$version"
  checked=$((checked + 1))
done
echo "release-notes: $checked changed version(s) checked against $base"
