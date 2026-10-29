#!/usr/bin/env bash
# The live labs' clean environment (the labs of 2026-10-28): runs the given
# command with only what a lab needs, never what the shell it was started
# from carries. Started from inside a Claude Code session, a lab inherited
# that session's markers and routing (CLAUDE_CODE_*, CLAUDECODE,
# ANTHROPIC_*): the tools it ran took themselves for that session's
# children, and their requests could follow the parent's routing rather
# than the lab account's own sign-in.
#
#   scripts/qa/lab_env.sh <command> [args...]
#
# Kept: the basics (HOME, PATH, USER, LOGNAME, SHELL, TMPDIR, LANG,
# LC_ALL, TERM), the build's (CARGO_*, RUSTFLAGS, RUSTUP_*, RUSTC_WRAPPER)
# and the lab's own (AIJ_LAB_*, AJ_LAB_OUT, DISABLE_AUTOUPDATER, which it
# sets to 1). Values are passed through, never printed.
set -euo pipefail
keep=(HOME PATH USER LOGNAME SHELL TMPDIR LANG LC_ALL TERM RUSTFLAGS RUSTC_WRAPPER AJ_LAB_OUT)
args=()
for name in "${keep[@]}"; do
  if [ -n "${!name+x}" ]; then args+=("$name=${!name}"); fi
done
while IFS='=' read -r name _; do
  case "$name" in
    CARGO_*|RUSTUP_*|AIJ_LAB_*) args+=("$name=${!name}") ;;
  esac
done < <(env)
exec env -i "${args[@]}" DISABLE_AUTOUPDATER=1 "$@"
