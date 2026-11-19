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

## Addendum: re-review of the fixes (2026-11-19, same reviewer, same rules)

- **Checked:** e424d3075 (I1), 7f1bb393c (M1, L1, L2, L3, I2, I3), b3167df27 and 33c4d68bf (M2), 0de22ddcd (ADR-0033 §4.2).
- **How:** by commit. I used `git archive 7f1bb393c` and `b3167df27` in the scratchpad, built with their own target (the helper builds with its synthetic-model tests; 8 pass and 2 are ignored), and ran my own C probes. I did not touch the working tree or `r05`.
- **What I could not run:** the app-side tests (`engines::local`), because a scratch `juggler-app` build would not fit the 33 GB that was free. The L3 pressure wiring is therefore verified by reading, plus the argv test.

### Verdicts

| Finding | Verdict | Evidence |
|---|---|---|
| **M1** sandbox too wide | **CLOSED** | Q1 below. The committed `sandbox.sb` denies by default, and every escape I tried fails under it while working without it. `MUT-SB1` (allowing `/private/tmp`) and `MUT-SB2` (allowing `network-outbound`) each make `under_its_sandbox_nothing_else_is_reachable` **fail**. `MUT-SB3` (allowing any exec) passes, because exec still needs `file-read` of the target; that is defence in depth, not a gap. |
| **M2** supply chain | **CLOSED** | Run offline on the archive: `cargo vet --locked --manifest-path crates/juggler-local-ai/Cargo.toml` gives "Vetting Succeeded (8 fully audited, 50 exempted)". `cargo deny --offline … check` gives advisories, bans, licenses and sources all ok. The build is `--offline --locked`, and CI and `just` run both. Q3 below covers the exemptions. |
| **L1** swap between hash and load | **CLOSED** (one test gap) | `O_NOFOLLOW`, then `fstat` device/inode/size matched against the app's hashed stamp, then llama.cpp reads `/dev/fd/<n>`. Removing `O_NOFOLLOW` (ML1a) and removing the identity check (ML1b) each **fail** `it_loads_only_the_hashed_plain_file_and_survives_junk`. Loading by name again instead of `/dev/fd` (ML1c) **passes**: no test pins the descriptor load. An in-place rewrite by a same-user process stays out of scope, as the ADR says. |
| **L2** special tokens in prompt text | **CLOSED** | Turning `plain_tokens` off for untrusted parts (ML2) **fails** `a_pasted_role_marker_stays_plain_text`. The sentinels are refused in input. Three residual details are under N4. |
| **L3** resources | **PARTIAL** | The critical-pressure rule is wired: no start or load, a running job is stopped every 250 ms, and the helper is dropped (read in code; `critical_memory_pressure_never_starts_a_model_and_stops_one_running`, not run). It acts at Critical, not at Warning as the plan said. **The memory cap is not enforced** (N1). |
| **I1** unsafe scan | **CLOSED** | Tokens are counted with `proc-macro2` (a dev-dependency). Each of these **fails** the test: `"a//b"; unsafe {}`, a second `allow(unsafe_code)`, and `unsafe` inside `macro_rules!`. `unsafe` in a comment or string passes, as it should. |
| **I2** `parse_request` without `check` | **CLOSED** | Dropping `check()` (MUT-P7 again) now **fails** `parse_request_checks_the_bounds`. |
| **I3** stale comments | **PARTIAL** | The script and ADR are corrected. `src/main.rs` (lines 14–15: "the App Sandbox it is signed with"), `build.rs` and `Info.plist` still describe the App Sandbox. |
| **I4** cut last line, lost cancel | **OPEN** (info) | It was not addressed. It remains harmless as described. |

### The coordinator's questions

1. **Is the profile deny-default? Yes.** I ran a probe of my own under the committed profile through `/usr/bin/sandbox-exec -p … -D HELPER/MODELS/CACHE`, against the same binary without the profile.

   | Probe | Under the profile | Without it |
   |---|---|---|
   | reads of `/private/tmp`, `$HOME`, `$HOME` via `/System/Volumes/Data`, `/etc/hosts`, `/private/etc/passwd`, `/Library/Preferences` | **all denied** | allowed |
   | file in MODELS | read allowed, **create denied** | both allowed |
   | create in CACHE | allowed (by design) | allowed |
   | create in `/private/tmp` | denied | allowed |
   | TCP connect to a harness listener; UDP `sendto`; `bind`/`listen`; connect to `/var/run/mDNSResponder` | **all denied**; the listener saw 0 connections | allowed; 1 connection |
   | `posix_spawn` of `/usr/bin/true` and `/usr/bin/open`; `fork` | **all denied** | allowed |
   | `LSOpenCFURLRef` | `-10661` (refused) | `-10814` (LaunchServices reached) |
   | `bootstrap_look_up` of launchservicesd, lsd.mapdb, pasteboard, windowserver, SecurityServer, cfprefsd (daemon and agent), distributed notifications, tccd, securityd.xpc, nsurlsessiond, dnssd | **all denied** | allowed |
   | `kill(other, 0)`; `task_for_pid` | denied; denied | allowed; denied |
   | `proc_listallpids`, `proc_pidinfo`/`proc_pidpath` of another process of the user's, `KERN_PROCARGS2` | **allowed** (N2) | allowed |

2. **Is relying on deprecated `sandbox-exec` acceptable?** Yes for a Developer ID app. It is still shipped, Apple's own tools and many third-party tools use it, and nothing replaces it short of the App Sandbox, which can't be narrowed. How each failure would show:
   - **The tool is removed.** `Local::command` returns `None` and the card says *HelperMissing*. This fails closed, but quietly: nothing says that the sandbox tool, not the helper, is what is missing.
   - **The profile is rejected** (an operation renamed or removed). This is tested: `sandbox-exec` exits 65 before the exec, the app gets no hello, and the engine is Failed/Unknown. This fails closed.
   - **Too little is allowed on a new OS** (for example a new Metal service). The helper fails to load and its jobs fall back. This fails closed.
   - **`sandbox-exec` still exists but stops applying the profile.** This would **fail open**: the helper never checks that it is sandboxed (N3). CI's `under_its_sandbox_nothing_else_is_reachable` would catch it on the runner's macOS only, not on a user's newer OS.

3. **Are the 50 exemptions real?** Mostly yes; I checked each claim mechanically against the app workspace.
   - **"Exempted at this version in the app's own workspace" (41 entries):** all true. Each is in the root `supply-chain/config.toml` at that exact version.
   - **Build-time only (`cc`, `cmake`, `find_cuda_helper`):** true.
   - **"Not in the app" (`encoding_rs`, `itoa`, `log`, `valuable`, `syn` 3.0.10):** all true: none is in the root lock. Two wordings are off, though:
     - `encoding_rs` is a *direct* dependency of the helper, not "transitive through llama-cpp-2";
     - `syn` 3 (pulled by `serde_derive` and `thiserror-impl` at build time) is not "small".
   - **`getrandom` 0.4.3 ("vetted there"):** true. Mozilla's imported audit covers it, so the exemption is redundant (`cargo vet prune`).
   - **What should worry you:** only the expected part. `llama-cpp-2` and `llama-cpp-sys-2` are exempted, not audited, so they are contained by the sandbox under D-131 rather than vetted. VENDORED.md records their `.crate` digests (they match the cache) and the vendored tree's digest. llama.cpp CVEs aren't in RustSec, so `cargo deny` won't flag them. The watch is the manual bump rule.
   - **Untidy, not a risk:** `supply-chain/audits.toml` is a copy of the app's sentry audits, which are irrelevant to the helper.
4. **Swap (L1):** yes. The check is real, and two mutations show it is tested (ML1a, ML1b). The `/dev/fd` load itself isn't pinned by a test (ML1c).
5. **Special tokens (L2):** yes, for control and user-defined tokens (ML2 fails). See N4 for the edges.
6. **Memory pressure (L3):** the pressure rule is wired (as read above). **The cap is not**: see N1.

### New findings

| # | Severity | Finding |
|---|---|---|
| **N1** | **Medium** | **The memory cap has no effect.** `taskpolicy -m` sets a jetsam limit on the process it spawns, but the next `exec` (`sandbox-exec` → helper) drops it. Tested with an allocator in the exact chain the app builds (`taskpolicy -c utility -m 64 sandbox-exec -p <committed profile> … alloc N`): 300 MiB and **2,000 MiB were touched without a kill**. Direct `taskpolicy -m 64 alloc 300` is killed (137), and a plain `sh -c exec` in between drops the limit too. Reversing the order works: `sandbox-exec … taskpolicy -c utility -m 64 alloc 300` is killed (137). That order needs the profile to allow exec and read of `/usr/sbin/taskpolicy` only. `the_helper_starts_capped_sandboxed_and_in_its_folder` checks only the argv, so it can't see this; add a test that allocates past the cap through the real chain. ADR-0033 §4.2 and VENDORED.md state the cap as a control, so correct them until it works. |
| **N2** | Low | Under the profile the helper can list processes and read other processes' paths and arguments (`proc_listallpids`, `proc_pidinfo`, `proc_pidpath`, `KERN_PROCARGS2`; macOS strips their environments, as tested). Arguments can carry secrets. The only way out is the helper's answers to the app. `(deny process-info-listpids)` closes the listing (tested). The other three were not closed by the `process-info*`/`sysctl-name-prefix` filters I tried. |
| **N3** | Low | **Fail-open if `sandbox-exec` ever runs without applying the profile.** The fix is a self-test at the helper's start, in safe Rust: open something the profile denies (`/etc/hosts`, or the parent of the models' folder), and exit before the hello if it opens. Separately, the "HelperMissing" reason could name the missing sandbox tool. |
| **N4** | Info | L2's edges: the special-token list leaves out `UNKNOWN`-attribute tokens, which llama.cpp's tokenizer also matches (`<unk>` stays parsable). A control token whose text is over 64 bytes is skipped (`token_to_piece_bytes(…, 64, …)`). Text without a marker takes the whole-string path, where a marker could in theory form across the template/content boundary (no ChatML case does). |

### Clean-up

Deleted: the archives (`rr`, `sc`), their targets, the probes (`p2`), the scan copies (`scan2`), the scratch TOML copies, and `$HOME/.aij-sbx-rereview-20261119`. Kept: nothing new. The leftover from the first pass, `~/Library/Containers/ai.figurehead.juggler.local-ai.review-probe/` (a 32 KB metadata plist, protected by `containermanagerd`), is still there for the maintainer to remove in Finder.

## Addendum 2: final check of 5e8999f8f, c9b19a0ad, 84a73c717

I checked these by commit (`git archive`) in the scratchpad, with their own targets. At 5e8999f8f the helper's suite passes: 3 unit tests, plus 11 integration tests with 2 ignored. I did not build the app's tests; the app-side items below are read and replicated.

| Item | Verdict | Evidence |
|---|---|---|
| **N1** memory cap | **CLOSED** | The chain is now `sandbox-exec <profile> taskpolicy -c utility -m <cap> <helper>` (`the_helper_starts_sandboxed_then_capped_and_in_its_folder` pins the argv). The profile allows exec and read of `/usr/sbin/taskpolicy` only. My own allocator through that exact chain and the v3 profile: 2,000 MiB under a 64 MiB cap is **killed (137)**; 300 MiB under a 4,096 MiB cap runs. The helper's `the_memory_cap_holds_under_the_sandbox` passes. |
| **N3** fail-open | **CLOSED** | The real helper at 5e8999f8f, run outside the profile, answers `hello … "sandboxed":false` and **exits 4 without serving**. Under the app's chain it says `sandboxed:true` and serves. The self-check is reading `/etc/hosts` and listing the folder above the models' folder. On the app side, by reading: a missing tool, a refused profile (exit 65, re-tried on `/usr/bin/true`) or `sandboxed:false` becomes a `Seen` sandbox fault. It is logged once with `tracing::error!` (target `ai`) and hidden as `HelperMissing`. `a_sandbox_that_can_t_be_applied_hides_the_jobs_and_says_why` covers all three with stand-in tools, checking hidden and never loaded. It does not assert the log line. The app never sets `UNSANDBOXED_FOR_TESTS` (asserted). |
| **N2** process listing | **PARTIAL** (an acceptable residual, if the docs say what it is) | `(deny process-info-listpids)` works: `proc_listallpids` gives `EPERM`. Two other routes still enumerate every process under the v3 profile. First, **`sysctl` `kern.proc.all`** returns all 838 processes with their names (657 of this user's). Adding `(deny sysctl-read (sysctl-name-prefix "kern.proc."))` closes it, and with that line the helper's whole suite still passes. Second, even with both denies, **brute-forcing `proc_pidpath` over pids 1–99,999 names all 838 in 0.01 s**. So "a known pid" is every pid. Arguments stay readable through `KERN_PROCARGS2`; environments are stripped by macOS. **Judgement:** this is acceptable as a residual. Without network or LaunchServices, the helper's only way out is its own answers to the app, and those are capped, strict and shown as summaries. Add the `kern.proc.` deny (free), and say in ADR-0033 §4.2 that the helper can learn every process's path and arguments, rather than that listing is denied. |
| **Trust test** | **CLOSED** | I copied `network_reached`, `network_reached_in`, `NETWORK_CRATES`, `network_allowed` and the test's dependency loop verbatim from c9b19a0ad into a scratch crate over an archive of the repo. As committed it passes. Adding `ureq` to the helper **fails** it ("juggler-local-ai can reach the network through juggler-local-ai > ureq"). Adding `ureq` to the protocol crate (the helper's path dependency) **fails** it too, through the protocol. Its reach is the crates in `NETWORK_CRATES` plus a source scan for std/tokio sockets. Socket code inside llama.cpp's C++ is caught by the build's `nm` import check and contained by the sandbox, not by this test. |
| **N4** token edges | **CLOSED** | `Unknown` tokens are now in the list, there is a no-cap piece read, and `a_marker_across_the_edge_of_untrusted_text_is_caught` passes. |
| **I3** stale comments | **CLOSED** | No "App Sandbox" wording remains in the helper's `src/`, `Info.plist` or examples. `build.rs` mentions it only to say it isn't used. |

Clean-up: I deleted the archives (`r3`, `trust`), their targets and the probes. Nothing new is kept.
