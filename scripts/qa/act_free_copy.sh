#!/usr/bin/env bash
# 0.4.3: an act-free copy of the installed AI Juggler's data, for the Dev
# build to run on (the real-Mac presence check, walks on real data). The
# Dev build then reads the real Apps, profiles and records as the installed
# one does, and acts on nothing.
#
#   scripts/qa/act_free_copy.sh hold      the Dev folder aside, an act-free
#                                         copy of the installed database in
#                                         its place
#   scripts/qa/act_free_copy.sh restore   the copy deleted, the Dev folder back
#
# Act-free: only juggler.sqlite is copied (a sqlite .backup of the live
# file, read-only), and in the copy keep alive is off on every App and the
# reset queue and the fleet outbox are empty. Pools are left as they are:
# no session talks to the Dev build's agent bridge, so they carry nothing
# (pausing them filled Needs you with a notice each, and pushed Open now
# out of the rail's drawn rows). `hold` prints the process ledger's row count, `restore`
# prints it again: it must stay 0 (nothing started). Quit the Dev build
# before either; check it started no child process while it ran
# (`pgrep -lP <pid>`).
set -euo pipefail
support="$HOME/Library/Application Support"
dev="$support/AI Juggler Dev"
held="$support/AI Juggler Dev (held for an act-free run)"
source_db="$support/AI Juggler/juggler.sqlite"
marker=".act-free-copy"

running() { pgrep -f "AI Juggler Dev.app/Contents/MacOS/" >/dev/null; }

case "${1:-}" in
  hold)
    running && { echo "act-free: quit the Dev build first" >&2; exit 1; }
    [ -e "$held" ] && { echo "act-free: a held Dev folder is there already; restore first" >&2; exit 1; }
    [ -f "$source_db" ] || { echo "act-free: no installed database" >&2; exit 1; }
    [ -e "$dev" ] && mv "$dev" "$held"
    mkdir -p "$dev"
    touch "$dev/$marker"
    sqlite3 "file:$source_db?mode=ro" ".backup '$dev/juggler.sqlite'"
    sqlite3 "$dev/juggler.sqlite" <<'SQL'
update apps set keep_alive = 0;
delete from reset_queue;
delete from fleet_outbox;
SQL
    echo "act-free: copy in place; process ledger rows: $(sqlite3 "$dev/juggler.sqlite" 'select count(*) from process_ledger')"
    ;;
  restore)
    running && { echo "act-free: quit the Dev build first" >&2; exit 1; }
    [ -f "$dev/$marker" ] || { echo "act-free: the Dev folder isn't an act-free copy; nothing deleted" >&2; exit 1; }
    echo "act-free: process ledger rows after the run: $(sqlite3 "file:$dev/juggler.sqlite?mode=ro" 'select count(*) from process_ledger')"
    rm -rf "$dev"
    [ -e "$held" ] && mv "$held" "$dev"
    echo "act-free: the copy is deleted and the Dev folder is back"
    ;;
  *)
    sed -n '2,20p' "$0" >&2
    exit 2
    ;;
esac
