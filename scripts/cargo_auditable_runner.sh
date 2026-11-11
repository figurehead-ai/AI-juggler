#!/usr/bin/env bash
# AJ-E2007 R5: Tauri's build runner (`cargo tauri build --runner …`).
# Every cargo build the release does goes through `cargo auditable`, which
# embeds the dependency list in the executable (a `.dep-v0` section that
# `cargo audit bin` and other scanners read).
set -euo pipefail
exec cargo auditable "$@"
