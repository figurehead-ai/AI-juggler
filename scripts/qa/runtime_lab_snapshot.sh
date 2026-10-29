#!/usr/bin/env bash
# AJ-E2301: before/after snapshot of tool folders for the runtime lab
# (docs/qa/runtime-lab.md). Metadata ONLY: each file's path, size, mode
# and modification time (`stat`), never its contents, so a credential file
# is listed by name and size and never read.
#
#   scripts/qa/runtime_lab_snapshot.sh take NAME DIR...   writes <out>/NAME.tsv
#   scripts/qa/runtime_lab_snapshot.sh diff BEFORE AFTER  what changed, grouped
#   scripts/qa/runtime_lab_snapshot.sh procs [PATTERN]    the lab's processes
#   scripts/qa/runtime_lab_snapshot.sh selftest           checks the locale rules
#
# Processes are printed as pid and program name only (`ps -o pid=,comm=`),
# never a command line: a full listing once showed another process's
# environment text (D-37). The pattern only selects pids (`pgrep -f`
# prints nothing else).
#
# The output folder is $AJ_LAB_OUT (default: /tmp/aj-runtime-lab). Nothing
# under the folders is written.
#
# Everything runs in the C locale, whatever the caller's (lab 2026-10-26):
# under a UTF-8 locale `pgrep -f` stops at a process whose arguments aren't
# UTF-8 ("illegal byte sequence"), and `sort` orders by the locale, so a
# `take` under one locale and a `diff` under another reported thousands of
# false changes.
set -euo pipefail
export LC_ALL=C
out="${AJ_LAB_OUT:-/tmp/aj-runtime-lab}"

# What the lab starts: AI Juggler's app-servers, headless claude, the
# bridge, AI Juggler Dev itself.
LAB_PATTERN='app-server --listen stdio://|claude -p --input-format stream-json|juggler-agent|target/debug/juggler-app'

# The processes matching $1 (default: the lab's), as "pid program", leaving
# out the apps' own (the Codex app's server lives inside /Applications).
# pgrep says 1 when nothing matches; anything above is a failure, and the
# list is then unknown: said, never printed as an empty list.
procs() {
  local pattern="${1:-$LAB_PATTERN}" pid pids status=0
  pids=$(pgrep -f "$pattern") || status=$?
  if [ "$status" -gt 1 ]; then
    echo "pgrep failed (exit $status): the lab's processes are unknown" >&2
    return 1
  fi
  for pid in $pids; do
    ps -o pid=,comm= -p "$pid" 2>/dev/null || true
  done | grep -v ' /Applications/' || true
}

take() {
  local name="$1"; shift
  local file="$out/$name.tsv"
  mkdir -p "$out"
  chmod 700 "$out"
  : > "$file"
  for dir in "$@"; do
    [ -d "$dir" ] || { echo "not a folder: $dir" >&2; exit 2; }
    # -P: never follow links; stat prints metadata only.
    find -P "$dir" -type f -print0 |
      xargs -0 stat -f $'%N\t%z\t%Sp\t%m' >> "$file"
  done
  sort -o "$file" "$file"
  echo "$(wc -l < "$file" | tr -d ' ') files -> $file"
  procs > "$out/$name.processes"
  echo "processes -> $out/$name.processes ($(wc -l < "$out/$name.processes" | tr -d ' '))"
}

diff_snapshots() {
  local before="$out/$1.tsv" after="$out/$2.tsv"
  echo "== added"
  comm -13 <(cut -f1 "$before") <(cut -f1 "$after")
  echo "== removed"
  comm -23 <(cut -f1 "$before") <(cut -f1 "$after")
  echo "== changed (size or time)"
  awk -F'\t' 'NR==FNR {seen[$1]=$2"\t"$4; next} ($1 in seen) && seen[$1] != $2"\t"$4 {print $1}' \
    "$before" "$after"
  echo "== processes still running from the lab"
  cat "$out/$2.processes" 2>/dev/null || true
}

# The locale rules hold, whatever locale this was started in.
selftest() {
  [ "$LC_ALL" = C ] || { echo "selftest: not the C locale" >&2; exit 1; }
  local sorted
  sorted=$(printf 'b\nB\na\n' | sort | tr '\n' ' ')
  [ "$sorted" = "B a b " ] || { echo "selftest: sort isn't bytewise: $sorted" >&2; exit 1; }
  procs 'aj-runtime-lab-selftest-matches-nothing' > /dev/null ||
    { echo "selftest: pgrep failed" >&2; exit 1; }
  echo "runtime_lab_snapshot selftest: ok"
}

case "${1:-}" in
  take) shift; [ $# -ge 2 ] || { echo "usage: take NAME DIR..." >&2; exit 2; }; take "$@" ;;
  diff) shift; [ $# -eq 2 ] || { echo "usage: diff BEFORE AFTER" >&2; exit 2; }; diff_snapshots "$@" ;;
  procs) shift; procs "$@" ;;
  selftest) selftest ;;
  *) sed -n '2,24p' "$0"; exit 2 ;;
esac
