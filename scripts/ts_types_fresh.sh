#!/usr/bin/env bash
# AJ-E2011 R4: the TypeScript types generated from Rust (ts-rs, written by
# each type's export_bindings test into ui/src/types/generated) match the
# Rust they come from. Run after the tests have run: a type changed in
# Rust without its file regenerated and committed, or a new one left
# uncommitted, fails here.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
changed="$(git status --porcelain -- ui/src/types/generated)"
if [ -n "$changed" ]; then
  echo "ts-types: the generated TypeScript types don't match the Rust (commit what the tests wrote):"
  echo "$changed"
  exit 1
fi
echo "ts-types: OK"
