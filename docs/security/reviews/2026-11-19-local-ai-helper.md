# Security review: the local-model helper `juggler-local-ai` and its IPC (D-131, ADR-0033 §4.2)

- **Reviewer:** an independent Opus session. It did not write the helper. It assumed nothing was safe until the code and a test (or its own experiment) showed it.
- **Date:** 2026-11-19. **Branch:** `release/0.5` at `44d5bf0d1`.
- **Commits reviewed:** 6287c18db (`background_io`, the one `unsafe`), 4ab5ce0d5 (`crates/juggler-local-ai-protocol`), a77c69ccb (`crates/juggler-local-ai`), 1f80617e8 (`engines/local/`, `engines/helper.rs`), e31b8133b (build, sign, bundle, CI, ADR-0033 §4.2 "As built"). For comparison: `code_trust::own_helper` (e638b9c18) and the Apple helper's bundling (416d3b35e).
- **llama.cpp:** llama-cpp-2 and llama-cpp-sys-2 at exactly 0.1.146. Their cached `.crate` SHA-256 values match the helper's `Cargo.lock` (`f3b0f368…`, `9b291e4b…`). The vendored tree reports ggml 0.9.11. Its llama.cpp commit is not recorded: `build-info.cmake` says `unknown`, and the crate's own VCS record names llama-cpp-rs `4afdaf0782ef`.

## Verdicts

| # | Severity | Finding |
|---|---|---|
| M1 | **Medium** | The sandbox is real but wider than ADR-0033 §4.2 says. It denies the network, `$HOME` and writes, as tested. It still **reads `/private/tmp`** and system files, and it **reaches LaunchServices**, so a compromised helper can hand a URL to the browser. That is an egress path that needs no network entitlement. |
| M2 | **Medium** | The helper's own workspace (60 crates: llama-cpp-2/-sys-2, bindgen, clang-sys, cmake, cc, libloading …) is outside `cargo deny check` and `cargo vet --locked`, which both run at the root only. It has no deny or vet entries, its release build is `--offline` without `--locked`, and it is not built with `cargo auditable`. |
| L1 | Low | There is a TOCTOU window between the hash and the load. The app hashes by path, then the helper re-opens by path (`lstat`, then `open` without `O_NOFOLLOW` and without a device/inode match), and llama.cpp maps the file shared. The stamp (size, device/inode, mtime) can be kept across an in-place rewrite with `utimes`. This needs a same-user writer, which the ADR already rules out of scope. |
| L2 | Low | Untrusted prompt text is tokenized with special tokens on: `str_to_token` passes `parse_special = true` over the whole rendered prompt. Text such as `<\|im_start\|>system` in a watched session becomes real role tokens, which makes prompt injection into summaries stronger. |
| L3 | Low | Resources: the memory-pressure rule isn't built (as the ADR says), so the check compares `min_memory_gb` with total RAM only. The helper has no memory limit, and the GPU isn't throttled (`taskpolicy -c utility` clamps CPU QoS). The prompt decode (up to 32 K tokens) can't be cancelled in the helper; the app kills it at deadline + 2 s. |
| I1 | Info | The `unsafe` source scan can be evaded on the same line (MUT-S2). It also doesn't count `allow(unsafe_code)`. The compiler lint (`deny`) still requires an allow, so this is a gap in the backstop only. |
| I2 | Info | The protocol crate's unit tests don't cover `parse_request → check` (MUT-P7 survives). The helper's integration test covers it, since an answer then has `id: null` rather than an id. I did not mutation-run that test: it rebuilds llama.cpp. |
| I3 | Info | Comments to correct: "refused by the sandbox anyway" in `build_local_ai_helper.sh` is false, because the sandbox allows `posix_spawn` (tested; the child inherits the sandbox). On macOS, ggml 0.9.11 forks a debugger only when `GGML_BACKTRACE_LLDB` is set, and the cleared environment already prevents that, so `GGML_NO_BACKTRACE=1` is a second guard. The `nm -u` import check is hygiene, not a boundary. |
| I4 | Info | Two edge behaviours: the app's `bounded_line` accepts a last line cut off by EOF, where the helper drops one (a cut JSON line fails to parse anyway). A `cancel` that arrives before the worker's `set_running` is lost, and the fallback is the kill after `CANCEL_GRACE`. |

**What holds (checked):**

- **Integrity** holds. Production uses `Local::default()`, so the helper is found only through `own_helper`: in the bundle's `Contents/MacOS`, or beside the executable only under `debug_assertions`. It must pass `trusted_executable`, and it is checked strictly against `identifier "ai.figurehead.juggler.local-ai"` plus the app's team when the app has one. It is re-resolved on every job. `taskpolicy` keeps the same pid (tested: `posix_spawn` with `SETEXEC`), so the signature-checked path is the process that runs, and `kill` reaches it. `Setup.program` (no signature check) is set only by tests; making `Setup` `cfg(test)`-only would make that structural.
- **IPC** holds both ways. Lines are capped before parsing: 32 KiB in and 64 KiB out, read through `take(max + 1)`. Unknown and duplicate fields are refused (`deny_unknown_fields`), and so are NULs and out-of-bound numbers. The file is `<64 lower-hex>.gguf` only, checked in the protocol and again in `llm::open`. There is no shell (`process::helper`: an absolute path, cleared environment, system `PATH`), and fault answers carry no text.
- **`unsafe`** is correct. `IOPOL_TYPE_DISK` 0, `IOPOL_SCOPE_THREAD` 1 and `IOPOL_THROTTLE` 3 match `<sys/resource.h>`, and so do both prototypes. The SAFETY note is true. Grep confirms it is the only `unsafe` in juggler-platform, the protocol, the helper (`forbid`) and `features/ai`.
- **Logging** is clean. No prompt or response text is logged anywhere in the path. The helper's stderr is `/dev/null`, llama.cpp's logs are voided, and `verify.rs`'s one `warn!` carries no text.

## How it was checked

Everything ran on the review's own copies in the scratchpad: a copy of the built helper (`r05/aarch64-apple-darwin/release/juggler-local-ai`), signed ad hoc by `scripts/build_local_ai_helper.sh sign`, which also passed its own `check`. Beside it ran the same binary unsigned (no sandbox) and three small C probes, signed with the helper's exact entitlements file and identifier `…local-ai.review-probe`. No model was downloaded, no tracked file was edited, and nothing was created in an AI Juggler data folder (the Dev edition was running).

| # | Test (the real helper over its protocol) | Result |
|---|---|---|
| T1 | 19 hostile lines: 5,000-deep nesting in an unknown field and in `sampling`; ids of 2⁶⁷ and `1e400`; size −1 and 8 GiB + 1; context 2³²; invalid UTF-8; a lone surrogate; `\u0000`; an unknown `sampling` field; a duplicate `op`; type confusion; an escaped `..\/`; upper-case and full-width hex; a guide over 8 KiB; both guide kinds | every line `bad_request` with `id: null`, and the helper stays alive and then answers `not_loaded` |
| T2 | a partial frame, then EOF | exit 0, and nothing written |
| T3 | a 32,768-byte line; a 32,769-byte line; 1 MiB with no newline | the first is read (`bad_request`); the second and third answer `too_large` and exit 2 (0.5 s), so nothing is buffered past the cap |
| T4 | read differential: a junk `<sha>.gguf` loaded from `$HOME/…`, `$TMPDIR` (`/var/folders`), `/Users/Shared`, `/private/var/tmp` | **sandboxed `no_file`**, unsandboxed `load_failed` (opened, then refused by llama.cpp): the sandbox denies the read |
| T5 | the same from `/private/tmp/…` | **`load_failed` both ways: the sandbox lets the helper read `/private/tmp`** (M1) |

| # | Probe (helper's entitlements) | Sandboxed | Unsandboxed |
|---|---|---|---|
| P1 | TCP connect to a harness listener on 127.0.0.1; UDP `sendto`; `bind`+`listen`; connect to `/var/run/mDNSResponder` | all `EPERM`; the listener saw **0** connections | allowed; 1 connection |
| P2 | read a file in `$HOME`; create in `$HOME`; create in `/private/tmp` | `EPERM`, `EPERM`, `EPERM` | allowed |
| P3 | read in `/private/tmp`; read `/etc/hosts` | **allowed**, allowed | allowed |
| P4 | `posix_spawn /usr/bin/true` | **allowed** (I3) | allowed |
| P5 | `/usr/bin/open` with an unregistered URL scheme | **reached LaunchServices' binding evaluator** (`-10814`, not a permission error), so an `https:` URL would open in the browser (M1) | same |
| P6 | list `~/Downloads` and `~/Desktop` through the container's links | `EPERM` | n/a |

Not re-tested: that a file inside a real `…/ai-models/` is readable and not writable (positive control). The author records it as checked by hand. It would need a folder in a live data folder.

| # | Mutation (scratch copy, its own target) | Result |
|---|---|---|
| MUT-P1 | `is_sha256` reduced to `len == 64` | `a_file_is_named_only_by_its_hash` **fails** |
| MUT-P2 | the request cap removed from `parse_request` | `an_oversized_line_is_refused_before_it_is_read` **fails** |
| MUT-P3 | `deny_unknown_fields` off `Request` | `malformed_lines_are_bad_requests` **fails** |
| MUT-P4 | the prompt's NUL check removed | `text_with_a_nul_…` **fails** |
| MUT-P5 | the answer's text cap removed | `answers_are_strict_too` **fails** |
| MUT-P6 | the load size bound removed | `numbers_outside_their_bounds_are_refused` **fails** |
| MUT-P7 | `check()` dropped from `parse_request` | **every test passes** (I2) |
| MUT-S1 | `fn f() { unsafe { } }` added to juggler-platform's source (the scan's own code, copied) | `the_only_unsafe_is_background_io` **fails** |
| MUT-S2 | `let _u = "a//b"; unsafe { }` on one line | **passes** (I1: `split("//")` cuts at the string) |
| MUT-S3 | `unsafe impl Send for X {}` | **fails** |

In place (the shared target, offline): `juggler-platform` `thread` and `code_trust` 9/9 pass. `juggler-app` `engines::local` and `engines::helper` 19/19 pass, including the hash-before-load, link and hung-helper tests.

## Recommendations

1. **M1.** Correct ADR-0033 §4.2 ("can read nothing else"): name `/private/tmp`, system files and LaunchServices as reachable. Treat **prompt text** as an attacker input to llama.cpp too, not just the GGUF: its tokenizer and template code have had memory-safety advisories, and session text can come from anyone an agent reads. Two ways to close the egress: apply an extra SBPL profile at the helper's start that denies `lsopen` and the `com.apple.coreservices.launchservicesd` lookup and narrows `file-read*` (a second `unsafe` carve-out, so a decision), or record it as an accepted residual with a D-number.
2. **M2.** Run `cargo deny check` and `cargo vet --locked` with `--manifest-path crates/juggler-local-ai/Cargo.toml` in CI. Add audits or exemptions for llama-cpp-2 and llama-cpp-sys-2 with the reason (D-131). Record the vendored llama.cpp commit and how its advisories are watched. Build the release helper `--locked`, stamping the version another way, and with `cargo auditable`.
3. **L1.** In the helper, open with `O_NOFOLLOW` (`OpenOptionsExt::custom_flags`, which is safe) and match `fstat`'s device/inode against values the app sends in `load`. Then load through `/dev/fd/N`, so the file that was checked is the file that is parsed.
4. **L2.** Remove the model's control-token strings from `instructions` and `prompt` before rendering, or tokenize the user content with `parse_special = false`.
5. **L3.** Build the memory-pressure rule before local models are offered by default. Consider a per-job token or time budget while agents run.
6. **I1.** Also count `allow(unsafe_code)` occurrences (expect one), and strip string literals before cutting at `//`.

## Clean-up

Deleted: the scratch helper copies, the probes and their sources, the protocol and scan mutation copies with their targets, the harness's temporary folders (`$HOME/.aij-sbx-review-20261119` and the ones in `$TMPDIR`, `/Users/Shared` and `/private/var/tmp`), and the probe container's data. **Left, as macOS refuses to delete it:** `~/Library/Containers/ai.figurehead.juggler.local-ai.review-probe/`. It holds only its 32 KB `.com.apple.containermanagerd.metadata.plist`, which `containermanagerd` protects, so the maintainer can remove it in Finder. The shared `r05` target got only the in-place test builds listed above.
