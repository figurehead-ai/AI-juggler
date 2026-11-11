#!/usr/bin/env bash
# Installs Debian packages on a CI runner, bounded and retried:
#
#   scripts/ci/apt_install.sh <package>...
#
# Run 43035537210 (#218): the runners lost DNS for every Ubuntu mirror for
# ~45 minutes. A plain `apt-get update` exits 0 on fetch failures (so the
# install then finds no packages), and `apt-get install` waits on every
# package in turn until the job's own timeout. Here an update that fetched
# nothing fails at once (--error-on=any), every fetch has a short timeout,
# and each attempt is capped; three attempts with a pause, then a clear
# error. The step's timeout-minutes is the outer bound.
set -euo pipefail
[ "$#" -gt 0 ] || { echo "usage: $0 <package>..." >&2; exit 2; }

SUDO=""
[ "$(id -u)" -eq 0 ] || SUDO="sudo"
OPTS=(-o Acquire::Retries=2 -o Acquire::http::Timeout=20 -o Acquire::https::Timeout=20)
export DEBIAN_FRONTEND=noninteractive

for attempt in 1 2 3; do
    if timeout 180 $SUDO apt-get "${OPTS[@]}" update -qq --error-on=any &&
        timeout 600 $SUDO apt-get "${OPTS[@]}" install -y -qq --no-install-recommends "$@"; then
        exit 0
    fi
    echo "::warning::apt attempt $attempt of 3 failed (mirror or DNS); retrying in $((attempt * 30)) s"
    [ "$attempt" -lt 3 ] && sleep $((attempt * 30))
done
echo "::error::apt could not install $* after 3 attempts: the runner's package mirrors are unreachable (see the Err:/Temporary failure lines above)"
exit 1
