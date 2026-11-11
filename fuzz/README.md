# Fuzzing

Parser fuzz targets (QUALITY.md §5). Targets are standalone cargo-fuzz
packages, built with nightly — they are intentionally outside the workspace.

```sh
rustup toolchain install nightly
cargo install cargo-fuzz
cd fuzz
cargo +nightly fuzz run transcript_parse -- -max_total_time=120
```

| Target | What it feeds | Must hold |
|---|---|---|
| `transcript_parse` | a tool's transcript lines | no panic |
| `link_frame_decode` | Fleet messages from a peer | no panic |
| `backup_staging` | a `.ajbackup` someone sent | no panic; nothing written outside staging |
| `cloud_container` | a downloaded cloud backup; a typed recovery code | no panic; a code that parses writes itself back |
| `pairing_messages` | a typed pairing code; the other side's SPAKE2 message and confirmation | no panic; a made-up confirmation never verifies |
| `shell_quoting` | any value quoted for a shell | the shell reads back exactly the value |

`.github/workflows/fuzz.yml` (AJ-E2008) runs each target for 4 minutes
every night on Linux, keeps its corpus in the Actions cache between runs,
and uploads a crashing input as an artifact. A crash is a release blocker
(RUNBOOK hard rails). A pull request touching `fuzz/` builds every target.
