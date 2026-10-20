#!/usr/bin/env bash
# AJ-E1010: checks a version's user-facing release notes before a release
# (docs/RELEASING.md): the file exists, starts with one plain sentence,
# uses only the allowed sections in order, and carries no task IDs, PR
# numbers, internal names or file paths. Usage: release_notes_check.sh X.Y.Z [FROM]
#
# When the database schema at X.Y.Z (its tag, or HEAD before it's tagged)
# is newer than at FROM — the version users come from; default: the
# newest earlier vX.Y.Z tag — the notes must carry a downgrade note under
# Known issues (scripts/downgrade_note.py).
set -euo pipefail
version="${1:?usage: release_notes_check.sh X.Y.Z [FROM]}"
file="docs/releases/${version}.md"
fail() { echo "release-notes: $file: $*" >&2; exit 1; }
[ -f "$file" ] || fail "missing — write the user-facing notes first (docs/RELEASING.md)"
first="$(grep -v '^\s*$' "$file" | head -1)"
case "$first" in
  "#"*|"-"*|"*"*) fail "the first line must be one plain sentence, not a heading or a bullet" ;;
esac
order=("## New" "## Improved" "## Fixed" "## Known issues")
last=-1
while IFS= read -r heading; do
  index=-1
  for i in "${!order[@]}"; do [ "${order[$i]}" = "$heading" ] && index=$i; done
  [ "$index" -ge 0 ] || fail "unexpected section \"$heading\" (allowed, in order: ${order[*]})"
  [ "$index" -gt "$last" ] || fail "sections out of order at \"$heading\""
  last=$index
done < <(grep '^## ' "$file")
if grep -nE 'AJ-E[0-9]+|(^|[^&])#[0-9]+\b|\bPR ?[0-9]+|crates/|ui/src|\.rs\b|\.tsx?\b|docs/' "$file"; then
  fail "remove task IDs, PR numbers and file paths — notes are for users"
fi
if grep -niE 'lossless' "$file"; then
  fail "never call switching lossless"
fi
grep -q '<[a-zA-Z]' "$file" && fail "raw HTML is not rendered on figurehead.ai — use Markdown"
ref="$(git rev-parse -q --verify "v${version}^{commit}" 2>/dev/null || git rev-parse HEAD)"
from="${2:-}"
if [ -z "$from" ]; then
  earlier="$( { git tag -l 'v[0-9]*' | sed 's/^v//' | grep -E '^[0-9]+\.[0-9]+\.[0-9]+$' | grep -vx "$version"; echo "$version"; } \
    | sort -V | awk -v v="$version" '$0 == v { print prev; exit } { prev = $0 }')"
  [ -n "$earlier" ] && from="v$earlier"
fi
if [ -n "$from" ]; then
  python3 "$(dirname "$0")/downgrade_note.py" check "$file" "$ref" "$from" || fail "needs a downgrade note"
else
  echo "downgrade-note: no earlier release to compare with"
fi
echo "release-notes: $file OK"
