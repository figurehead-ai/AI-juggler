#!/usr/bin/env bash
# Installs Debian packages on a CI runner, bounded, and on another mirror
# when the runner's own one fails:
#
#   scripts/ci/apt_install.sh <package>...
#
# Why (#218, runs 43035537210 and 43053987177): GitHub's runners fetch from
# Azure's regional Ubuntu mirror (azure.archive.ubuntu.com, through
# /etc/apt/apt-mirrors.txt). It can stall for many minutes or stop
# resolving while the rest of the network works (actions/runner-images
# #14823, #15142; the same on 24.04, so pinning the image doesn't help).
# apt's own timeouts only fire when no byte arrives, and a failed update
# still exits 0, so a plain `apt-get update && apt-get install` either sits
# until the job's timeout or installs nothing.
#
# Each attempt uses one mirror: the runner's own, then Canonical's archive,
# then kernel.org's (another domain and network). Every attempt is capped,
# an update that fetched nothing fails it (--error-on=any), and apt's own
# lines stay in the log so a failure shows which host did what.
set -euo pipefail
[ "$#" -gt 0 ] || { echo "usage: $0 <package>..." >&2; exit 2; }

SUDO=""
[ "$(id -u)" -eq 0 ] || SUDO="sudo"
export DEBIAN_FRONTEND=noninteractive
OPTS=(-o Acquire::Retries=1 -o Acquire::http::Timeout=15 -o Acquire::https::Timeout=15)
MIRRORS=("" "http://archive.ubuntu.com/ubuntu/" "https://mirrors.edge.kernel.org/ubuntu/")
# The files that name a mirror: the runner image's list, and the plain
# sources (a container image has only those).
FILES=$(ls /etc/apt/apt-mirrors.txt /etc/apt/sources.list /etc/apt/sources.list.d/*.sources \
    /etc/apt/sources.list.d/*.list 2>/dev/null || true)

echo "::group::apt: the runner's image and package sources"
{ lsb_release -ds 2>/dev/null || head -2 /etc/os-release; } || true
for f in $FILES; do echo "--- $f"; grep -vE '^\s*(#|$)' "$f" || true; done
for h in azure.archive.ubuntu.com archive.ubuntu.com security.ubuntu.com mirrors.edge.kernel.org; do
    echo "resolve $h: $(timeout 10 getent hosts "$h" | head -1 || echo 'no answer')"
done
echo "::endgroup::"

# Points every Ubuntu source at one mirror ($1), keeping suites and parts.
use_mirror() {
    local m=$1 f
    for f in $FILES; do
        if [ "$f" = /etc/apt/apt-mirrors.txt ]; then
            echo "$m" | $SUDO tee "$f" > /dev/null
        else
            $SUDO sed -E -i "s#https?://([a-z0-9.-]+\.)?(archive|security)\.ubuntu\.com/ubuntu/?#${m%/}#g; s#https?://mirrors\.edge\.kernel\.org/ubuntu/?#${m%/}#g" "$f"
        fi
    done
}

attempt=0
for mirror in "${MIRRORS[@]}"; do
    attempt=$((attempt + 1))
    if [ -n "$mirror" ]; then
        echo "apt: attempt $attempt of ${#MIRRORS[@]} uses $mirror"
        use_mirror "$mirror"
    else
        echo "apt: attempt $attempt of ${#MIRRORS[@]} uses the runner's own mirrors"
    fi
    status=0
    timeout 150 $SUDO apt-get "${OPTS[@]}" update --error-on=any &&
        timeout 600 $SUDO apt-get "${OPTS[@]}" install -y -q --no-install-recommends "$@" ||
        status=$?
    [ "$status" -eq 0 ] && exit 0
    echo "::warning::apt attempt $attempt failed (status $status; 124 = it hit its time limit)${mirror:+ on $mirror}; the lines above show which host failed"
done
echo "::error::apt could not install $* from any of the runner's, Canonical's or kernel.org's Ubuntu mirrors (each attempt's lines are above)"
exit 1
