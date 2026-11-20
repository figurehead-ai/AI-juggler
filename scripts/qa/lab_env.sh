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
# LC_ALL, TERM), the build's (CARGO_*, RUSTFLAGS, RUSTUP_*, RUSTC_WRAPPER),
# the test bridge's switches (AIJ_DEV_BRIDGE*), the lab's own (AIJ_LAB_*, AIJ_QA_* seeding, AJ_LAB_OUT, DISABLE_AUTOUPDATER, which it
# sets to 1), the tools' stand-ins it names (AI_JUGGLER_CLAUDE_BIN,
# AI_JUGGLER_CODEX_BIN), and on a dry run the stand-ins' AI_JUGGLER_*
# overrides (below). Values are passed through, never printed.
set -euo pipefail
# 0.4.6 (the presence report): proof A's headless `claude -p`, typed by hand
# inside a Code-tab session, inherited CLAUDE_CODE_ENTRYPOINT=claude-desktop
# and wrote 5 "desktop" transcripts into the Lab App's folder. Every tool a
# lab run starts goes through this script; `--selftest` proves the session
# markers never reach it.
if [ "${1:-}" = "--selftest" ]; then
  out="$(CLAUDE_CODE_ENTRYPOINT=claude-desktop CLAUDECODE=1 CLAUDE_CODE_SSE_PORT=1 ANTHROPIC_BASE_URL=x CLAUDE_CONFIG_DIR=/nowhere "$0" /usr/bin/env)"
  if echo "$out" | grep -qE '^(CLAUDE_CODE_|CLAUDECODE|ANTHROPIC_|CLAUDE_CONFIG_DIR)'; then
    echo "lab_env selftest: a session marker got through" >&2; exit 1
  fi
  echo "$out" | grep -q '^DISABLE_AUTOUPDATER=1$' || { echo "lab_env selftest: DISABLE_AUTOUPDATER not set" >&2; exit 1; }
  # The lab's stand-ins reach the app (lab_scenario.sh open names them).
  out="$(AI_JUGGLER_CLAUDE_BIN=/tmp/aij-lab-x/.aij-lab-bin/claude "$0" /usr/bin/env)"
  echo "$out" | grep -q '^AI_JUGGLER_CLAUDE_BIN=/tmp/aij-lab-x/.aij-lab-bin/claude$' || { echo "lab_env selftest: the stand-in's path was dropped" >&2; exit 1; }
  echo "lab_env selftest: ok"; exit 0
fi
keep=(HOME PATH USER LOGNAME SHELL TMPDIR LANG LC_ALL TERM RUSTFLAGS RUSTC_WRAPPER AJ_LAB_OUT)
args=()
for name in "${keep[@]}"; do
  if [ -n "${!name+x}" ]; then args+=("$name=${!name}"); fi
done
while IFS='=' read -r name _; do
  case "$name" in
    CARGO_*|RUSTUP_*|AIJ_LAB_*|AIJ_QA_*|AIJ_DEV_BRIDGE|AIJ_DEV_BRIDGE_ORIGIN) args+=("$name=${!name}") ;;
    # 0.4.7: the lab's stand-ins for the tools (lab_standin.sh), which
    # lab_scenario.sh names: a lab never runs the real ones.
    AI_JUGGLER_CLAUDE_BIN|AI_JUGGLER_CODEX_BIN) args+=("$name=${!name}") ;;
  esac
done < <(env)
# A dry run's stand-in tools (AJ-E1612, lab_scenario.sh --standins): the
# adapters' test overrides pass only while AIJ_LAB_STANDINS names a lab's
# stand-ins folder, and only those that point into it.
case "${AIJ_LAB_STANDINS:-}" in
  /tmp/aij-lab*/standins|/private/tmp/aij-lab*/standins)
    for name in AI_JUGGLER_OPENCODE_BIN AI_JUGGLER_KIMI_BIN AI_JUGGLER_GROK_BIN AI_JUGGLER_MUSE_BIN AI_JUGGLER_HERMES_BIN AI_JUGGLER_ZCODE_ENGINE; do
      value="${!name:-}"
      case "$value" in
        "$AIJ_LAB_STANDINS"/*) [[ "$value" == *..* ]] || args+=("$name=$value") ;;
      esac
    done ;;
esac
exec env -i "${args[@]}" DISABLE_AUTOUPDATER=1 "$@"
