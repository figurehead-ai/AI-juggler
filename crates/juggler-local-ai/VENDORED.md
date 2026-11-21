# What `juggler-local-ai` builds from (D-131; the 2026-11-19 security review's M2)

The helper's only third-party code that runs is llama.cpp, through two crates pinned exactly. Both come from crates.io. They are built offline and locked (`--offline --locked`), from the cargo cache, by `scripts/build_local_ai_helper.sh`.

| Crate | Version | `.crate` SHA-256 (matches this folder's `Cargo.lock`) |
|---|---|---|
| llama-cpp-2 | 0.1.146 | `f3b0f368c76cc0fe475e8257aeeec269e0d6569bd48b1f503efd0963fc3ee397` |
| llama-cpp-sys-2 | 0.1.146 | `9b291e4bc2d10c43cd8dec16d49b6104cb3cb125f596ec380a753a5db1d965dd` |

## The vendored llama.cpp and ggml

- **ggml 0.9.11.** This comes from `llama.cpp/ggml/CMakeLists.txt` in llama-cpp-sys-2 0.1.146: `GGML_VERSION_MAJOR 0`, `MINOR 9`, `PATCH 11`.
- **The upstream commit, as far as the published crate records it:** llama-cpp-rs `4afdaf0782ef7f3254a186a7ff67a1c7491c6dce`. This is from the crate's `.cargo_vcs_info.json`. That repository vendors llama.cpp as a git submodule, and the submodule's own commit isn't in the published crate:
  - `llama.cpp/cmake/build-info.cmake` sets `BUILD_NUMBER 0`, so `BUILD_COMMIT` is unknown;
  - no `.git` folder ships in the crate.
- **The vendored tree's digest** identifies those exact sources instead: `316f0c79ca0e700fad3ce458dc2da353ee25c38d685f0b651f5abd65597ad9ac`, over 859 files. It is the SHA-256 of `shasum -a 256` over every file under `llama-cpp-sys-2-0.1.146/llama.cpp`, sorted by path (`find . -type f | LC_ALL=C sort | xargs shasum -a 256 | shasum -a 256`).

## How advisories are watched

- **`cargo deny check`** runs on this workspace with `--manifest-path`, under the root `deny.toml`, in `just verify-static` and CI's static-rust job. RustSec advisories for llama-cpp-2 and llama-cpp-sys-2 fail it. llama.cpp's own CVEs (GitHub security advisories on ggml-org/llama.cpp) aren't in RustSec.
- **Each version bump of these two crates is a decision.** It needs:
  - the maintainer's yes;
  - a check of llama.cpp's advisories since ggml 0.9.11;
  - a new row in this file.
- **Until then, what limits a flaw:** the helper's sandbox. It is denied by default:
  - no network;
  - no file outside the models' folder and the system's libraries;
  - no other program but the task-policy tool that starts it;
  - no service but Metal's compiler.

  The helper also has a memory cap, set after the sandbox's exec so that it holds (tested), and runs at utility QoS (ADR-0033 §4.2, "As built").

## How the rest is vetted

`cargo vet --locked --manifest-path crates/juggler-local-ai/Cargo.toml` uses `supply-chain/` in this folder:
- the app workspace's audits and imported records;
- one exemption per remaining crate, each with its reason.
