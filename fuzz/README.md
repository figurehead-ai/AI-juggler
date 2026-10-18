# Fuzzing

Parser fuzz targets (QUALITY.md §5). Targets are standalone cargo-fuzz
packages, built with nightly — they are intentionally outside the workspace.

```sh
rustup toolchain install nightly
cargo install cargo-fuzz
cd fuzz
cargo +nightly fuzz run transcript_parse -- -max_total_time=120
```

Corpora are gitignored until a curated seed corpus is committed; crashes are
release blockers (RUNBOOK hard rails). CI gains a scheduled fuzz smoke build
with the Gate V1 report.
