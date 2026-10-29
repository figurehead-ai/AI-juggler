# Windows and Linux readiness audit

Research date: 2026-10-27
Code read: `origin/release/0.2` at `41f6c8c3` (0.2.0 line, 280 commits ahead of `main`).
Method: read-only. Nothing was compiled, run or signed on Windows or Linux, and no real
account, credential or AI Juggler data was opened. Every code claim below comes from
reading the file at the line given. Claims about the vendors and the platform stacks
carry the labels of `docs/research/EVIDENCE-STANDARD.md`:

| Label | Meaning here |
|---|---|
| **VP** | Verified-primary: the vendor's or crate's own docs/source, read on the date shown |
| **VS** | Verified-secondary: a second-hand source |
| **NE** | Negative-evidence: searched, not found (search log in the appendix) |
| **I** | Inferred: our reasoning. Never product fact |
| **U** | Unknown: needs a lab on that OS |

The maintainer's ruling of 2026-10-27 (memory: "Windows and Linux soon") is the brief:
keep macOS specifics behind OS seams so a Windows and a Linux release stay cheap.

---

## 0. The short version

1. **The repo is already half-seamed, and the seams are mostly honest.** The supervisor
   (`group.rs`, `ports.rs`, `awake.rs`, `zombies.rs`, `machine.rs`), the credential
   stores, `os_access`, `activation`, `preview`, `app_icons`, `login_item` and
   `handover` all have a `#[cfg]` module with a non-macOS stub that says "not here"
   rather than lying. This is the pattern to copy.
2. **Linux is much closer than Windows.** Everything Unix compiles and mostly works on
   Linux: sockets, process groups, signals, modes, ports, PTYs. What is missing on
   Linux is *features* (Go to it, keep-awake, login item, credentials, app detection,
   packaging), not correctness.
3. **Windows has compile blockers on `release/0.2` and silent wrong answers behind them.**
   - **Blocker (by reading, not built):** three modules added after `main`
     (`features/agent_bridge/{mod,server}.rs`, `features/runtime/launch.rs`) use
     `std::os::unix`, `tokio::net::UnixListener` and `file_trust::me()` with no `cfg`,
     and `file_trust::me()` exists only under `cfg(unix)`. `main`'s `build-windows` job
     is green (run 40488659421, 2026-10-27), but those modules are not on `main`.
     The first action is to run that job on a `release/0.2` PR (U: I found no CI run
     for the branch).
   - **False success:** on non-Unix, `signal_pid` returns `false` for every signal, so
     `member_gone` reads every process as already gone and the stop ladder reports
     "stopped" while the process lives (`crates/juggler-supervisor/src/tree.rs:674-717`,
     `800-820`). This is worse than a missing feature: it can show a kill that did not happen.
   - **Silent recognition failures:** `claude.exe` / `codex.exe`, `\` separators,
     `USERPROFILE` and `.cmd` shims are not handled in process and tool detection.
4. **The same five leaks repeat across the codebase**, which is why a few seams unlock a lot:
   *executable identity* (names, `.exe`, separators, `.app` bundles), *home and tool-home
   resolution* (three different resolvers), *owner-only files and local IPC*
   (`std::os::unix` in about 40 lines), *"open / reveal / run this"* (`/usr/bin/open`,
   `osascript`, `lsof`, `ps` by absolute path), and *Mac words in user-facing text*
   (about 210 lines in `ui/`, about 90 in Rust).
5. **What the vendors store is mostly the same everywhere** (VP): Claude Code, Codex,
   Kimi Code and Grok Build all keep a dot-folder under the user's home on all three OSes,
   so the adapters' home paths carry over. The differences that matter are credentials
   (files, not Keychain, off macOS), the desktop apps (different data dirs and process
   names, mostly **U**), and WSL (a Windows-only concept AI Juggler has no model for).

The ranked plan is in section 5. The minimum for a first Linux build and for a first
Windows build is in sections 5.3 and 5.4.

---

## 1. How to read the inventory

Guard status:

| Tag | Meaning |
|---|---|
| **seam-real** | `cfg` split with a real non-macOS implementation |
| **seam-stub** | `cfg` split; the non-macOS branch honestly reports "unsupported" / empty |
| **seam-false** | `cfg` split whose non-macOS branch misreports (success, wrong copy) |
| **cfg-unix** | `cfg(unix)` with a non-Unix stub (macOS and Linux both take the real branch) |
| **scattered** | no `cfg`; the assumption is inline |
| **gap** | `cfg` on one side, nothing on the other (won't compile, or feature silently absent) |

Severity is per OS: **B** blocker (won't compile or start), **H** high (a core feature is
absent or answers wrongly without saying so), **M** medium (degraded), **L** low
(cosmetic, test-only, or fails honestly). `Lin` / `Win` columns; "–" means works as is.
Paths are relative to the repo root; line numbers are at `41f6c8c3`.

Test-only code (fixture strings, `/bin/sh`, `/Applications/...` in `#[cfg(test)]`
modules and `crates/*/tests`, `bin/fake_*.rs`) is left out of the tables on purpose. It
assumes Unix throughout: `juggler-supervisor/tests/{supervisor,collateral,fleet_identity,node_script_tool}.rs`
are `#![cfg(unix)]`, `awake_pmset.rs` is macOS-only, and
`juggler-app/tests/activation_real.rs` is macOS-gated. A Windows test run needs a plan
(section 5.4), not just a green `cargo build`.

---

## 2. Inventory by concern

### C1. App data and tool-home locations

| # | file:line | Assumption | Guard | Lin | Win |
|---|---|---|---|---|---|
| 1.1 | `crates/juggler-app/src/state.rs:573-577` | App data is `dirs::data_dir()/<edition>` | seam-real (the `dirs` crate) | – | **M**: Windows `data_dir` is Roaming `%APPDATA%` (VP, docs.rs/dirs, 2026-10-27). A SQLite store plus session logs in Roaming is wrong (roams on domain profiles, syncs big files). `data_local_dir` is the right base. Moving it after release is a migration, so decide before the first Windows build |
| 1.2 | `crates/juggler-adapters/src/inventory_files.rs:174-178`, `claude_code.rs:1223-1230` | Home = `HOME`, else `USERPROFILE` | scattered | – | L (Git Bash can set `HOME` to another folder than `USERPROFILE`) |
| 1.3 | `crates/juggler-adapter/src/user_path.rs:105` | Home = `HOME` only | scattered | – | **M**: no home → no `~/.cargo/bin`, nvm, npm-global dirs |
| 1.4 | `crates/juggler-app/src/crash.rs:222,715`, `machine_health.rs:381,1144`, `commands.rs:4883`, `features/conversations/mod.rs:596`, `features/quota/records.rs:77`, `features/app_icons/bundle.rs:83` | Home = `dirs::home_dir()` | scattered | – | L→**M**: a third resolver. Grok Build deliberately does *not* use `dirs::home_dir()` because on Windows it ignores a redirected `USERPROFILE` (VP, xai-org/grok-build `xai-dirs/src/lib.rs`, 2026-10-27), so tool and app can disagree on which home is "home" |
| 1.5 | `crates/juggler-adapters/src/claude_code.rs:352,1149-1157,1205-1220` | `~/.claude` and `~/.claude.json`; `trim_end_matches('/')` | scattered | – | L (matches Claude's own `%USERPROFILE%\.claude`, VP 2026-10-27; the `'/'` trim is moot because `Path` compares components) |
| 1.6 | `crates/juggler-adapters/src/codex.rs:436,727`, `codex_environment.rs:91,99`; `kimi_code.rs:127,133`, `kimi_environment.rs:95,103`; `grok_build.rs:149,154`, `grok_environment.rs:82,90` | `~/.codex`, `~/.kimi-code`, `~/.grok` | scattered | – | L (same on Windows for all three, see section 4) |
| 1.7 | `crates/juggler-adapters/src/opencode.rs:195-210,866`, `opencode_environment.rs:116-123,166` | OpenCode data `~/.local/share/opencode`, config `~/.config/opencode`, hardcoded; `XDG_*` only an extra scan candidate (consts at `opencode.rs:70-74` used for managed launches) | scattered | **M** when the user exports `XDG_DATA_HOME` / `XDG_CONFIG_HOME` | L: OpenCode's own Windows path is `%USERPROFILE%\.local\share\opencode` (VP, opencode.ai/docs/troubleshooting, 2026-10-27), which the hardcode happens to match |
| 1.8 | `crates/juggler-adapters/src/claude_code.rs:267-273,309,1397` | t3.codes profile roots under `~/Library/Application Support/ClaudeCodeProfiles`; `"/ClaudeCodeProfiles/"` substring | scattered | L (absent) | L (absent; `/` separators) |
| 1.9 | `crates/juggler-adapters/src/other_managers.rs:178-180` | vibe-kanban lists macOS, Linux and Windows dirs | seam-real (the only multi-OS location table in the repo) | – | – |
| 1.10 | `crates/juggler-adapters/src/claude_live.rs:226-239,152-153` | Claude desktop host: `default_instance = "Library/Application Support/Claude"`, sessions under `<instance>/claude-code-sessions` | scattered | – (no official Linux data dir known, **U**) | **H**: Claude's Windows desktop data is reported at `%APPDATA%\Claude` (VS, 2026-10-27), so its sessions are never found. Also feeds `quota/records.rs:77-95` |
| 1.11 | `crates/juggler-adapter/src/handover.rs:236,346-347,365,407` | `/`-separated cwds; `mangled_dir` replaces `/`/`\` with `-` as the project-folder name | scattered | – | **M**: `C:\x` cwds are not remapped; how Claude mangles a Windows cwd is **U** |
| 1.12 | `crates/juggler-app/src/crash_native.rs:36-43` | `~/Library/Logs/DiagnosticReports` | seam-stub (`None` elsewhere) | L | L (no native crash capture off-Mac; Rust-panic reports are portable) |
| 1.13 | `crates/juggler-app/src/machine_memory.rs:43,54,120-126` | Bundle names and `/Applications/Docker.app/...` helper paths; `rsplit('/')` | scattered | L | L |

### C2. Process listing, identity and role detection (sysinfo)

`sysinfo` 0.39 is cross-platform (VP, docs.rs/sysinfo, 2026-10-27). The leak is how its
answers are *interpreted*.

| # | file:line | Assumption | Guard | Lin | Win |
|---|---|---|---|---|---|
| 2.1 | `crates/juggler-supervisor/src/tree.rs:451-474,488-503` | Recorded `exe` string equals the live `process.exe()` | scattered | M: `/proc/<pid>/exe` appends `" (deleted)"` and is `None` for other users | M: `exe()` can be `None` without privilege; case and `\\?\` prefixes differ. Fails closed (`Unverifiable`), so over-refuses rather than killing the wrong thing |
| 2.2 | `crates/juggler-supervisor/src/live_stats.rs:772-780` | `row.exe_path.starts_with('/')` before comparing | scattered | – | **M**: always false on `C:\...`; the exe leg of helper identity silently turns off (start time still verifies) |
| 2.3 | `live_stats.rs:134-136,1139-1147,1429-1448`; `orphans.rs:197` | `name()` / `exe()` / space-joined `cmd()` treated as ps-style | scattered | M: Linux `name` is `comm`, capped at 15 chars | M: names carry `.exe`; space-joined quoted paths are ambiguous |
| 2.4 | `crates/juggler-app/src/features/processes/roles.rs:102-106` (`program_name`); `commands.rs:8093,8658,8702-8705,8982,10012,10063,11244`; `features/pools/cards.rs:284-285`; `stop.rs:134` | Splits on `/` only, no `.exe` strip; compares to bare `juggler-agent`; shell list is `sh bash zsh fish` | scattered | – | **M**: `guess_role`, helper names, `exe_ok` and the `PROGRAM` check degrade; `juggler-agent.exe` never equals `juggler-agent`; `cmd` / `powershell` not recognised as shells |
| 2.5 | `crates/juggler-adapters/src/claude_code.rs:1437-1456` (`is_cli_process`); `codex.rs`, `codex_conversations.rs:243`; `codex_live.rs:60-65` | Binary name is exactly `claude` / `codex`; `cmd.starts_with('/')` | scattered | – | **H**: `claude.exe` / `codex.exe` never match. CLI sessions are not recognised (This Mac, live conversations, running setups) |
| 2.6 | `crates/juggler-app/src/features/conversations/places.rs:94-110,154-162,207`; `running_setups.rs:46-58`; `stop.rs:130` | An app is the outermost `*.app` path component; Claude desktop is `…/Claude.app/Contents/MacOS/Claude` | scattered | **M**: never matches, so "other apps" falls back to the process name; the Claude desktop place is never detected | **M**: same |
| 2.7 | `crates/juggler-supervisor/src/live_stats.rs:115` | `ppid == 1` means re-parented to init/launchd | scattered | L (user-session reapers) | L (no re-parenting; falls into the "left the table" branch, still correct) |
| 2.8 | `crates/juggler-app/src/features/processes/stop.rs:101` | `pid <= 1` is launchd | scattered | – | L |
| 2.9 | `crates/juggler-supervisor/src/zombies.rs:155-196` | Spawns `/bin/ps -A -o pid=,ppid=,stat=,etime=,ucomm=` | seam-stub (`read_ps` → `None`) | – (sysinfo reports zombies itself, `live_stats.rs:138,1447`) | L (no zombies) |
| 2.10 | `crates/juggler-app/src/crash.rs:225` | User-name scrub reads `env::var("USER")` | scattered | – | **M** (privacy): `USER` is unset on Windows, so the user name is not scrubbed from reports (the home-path scrub still works) |

### C3. Stopping, process groups and trees (signals)

| # | file:line | Assumption | Guard | Lin | Win |
|---|---|---|---|---|---|
| 3.1 | `crates/juggler-supervisor/src/tree.rs:800-820` (`signal_pid`, documented as "the ONE platform seam") | `nix::kill`: SIGTERM / SIGKILL / signal 0 as `Probe` | cfg-unix; **non-Unix returns `false` for all signals, `Probe` included** | – | **H** |
| 3.2 | `tree.rs:674-717` (`member_gone`, `terminate_group`); callers `commands.rs:9253`, `stop.rs:361` | `!signal_pid(Probe)` means the process is gone | **seam-false** with 3.1: every member reads gone → `AlreadyGone`, `all_succeeded = true`, nothing killed | – | **H** (reports success without killing) |
| 3.3 | `supervisor.rs:795-815,1531-1542` | `send_terminate` = SIGTERM to the root, then grace, then `force_kill` | cfg-unix; non-Unix `send_terminate` is a no-op, so `stop` waits the full grace then kills the **root only** (`Child::kill` / portable-pty killer = TerminateProcess). Descendants survive | – | **H** |
| 3.4 | `supervisor.rs:1225-1232`; `stop_guard.rs:28,44-51` | Adopted-root kill and "pid still exists" via `signal_pid` | inherits 3.1 | – | **H** (an adopted session cannot be killed) |
| 3.5 | `group.rs:55-91` (`make_group_leader`, `pgid_of`), `piped.rs:199` | `process_group(0)`; `getpgid` | cfg-unix; non-Unix `Err(UnsupportedPlatform)`; the caller prints `eprintln!` on every piped spawn and goes on groupless. The doc comment says a Job Object "lands separately" | – | **H**: no Windows tree-kill exists anywhere |
| 3.6 | `tree.rs:953-969` (`pgid_members`) | Full sysinfo scan plus `getpgid` | cfg-unix → empty on Windows | – | **H** |
| 3.7 | `supervisor.rs:1520-1529` | `ExitStatusExt::signal()` | cfg-unix; non-Unix → `Signaled(None)` | – | L |
| 3.8 | `supervisor.rs:663-669` | PTY exit is a single `i32` | portable | – | L: NTSTATUS codes overflow to `i32::MAX` |
| 3.9 | `tree.rs:655-661` vs `stop.rs:287` | Ladder label by `cfg!` in two places | scattered | – | L: the Windows label says "TerminateProcess only", which the code does not implement |

### C4. PTY, spawn, PATH and environment

| # | file:line | Assumption | Guard | Lin | Win |
|---|---|---|---|---|---|
| 4.1 | `crates/juggler-supervisor/src/supervisor.rs:597-650` | `portable-pty` `NativePtySystem` | portable (no cfg) | – | M: ConPTY behind the crate (I: wezterm's crate; the docs.rs page confirms only that Windows has "a variety of implementations", VP 2026-10-27); `PATH` vs inherited `Path` key handling **U** |
| 4.2 | `crates/juggler-adapter/src/user_path.rs:44-54` (`ENV_ALLOWLIST`), `136-143,264-270` | Probes and the login shell run under `env_clear()` plus `HOME USER LOGNAME SHELL TMPDIR LANG LC_ALL LC_CTYPE __CF_USER_TEXT_ENCODING` | scattered | L-M: `XDG_*`, `DISPLAY`, `WAYLAND_DISPLAY` dropped, so tools that read `XDG_CONFIG_HOME` misbehave | **H**: no `USERPROFILE`, `SystemRoot`, `APPDATA`, `LOCALAPPDATA`, `PATHEXT`, `TEMP`, so `claude --version` / `node` probes likely fail |
| 4.3 | `user_path.rs:35-61,201,239-243,268` | Login shell `-l` with a POSIX `printf` on stdin; fallback `/bin/zsh`; `SYSTEM_PATH=/usr/bin:/bin:/usr/sbin:/sbin`; `/opt/homebrew/bin`; `.npm-global/bin` | scattered | L: falls back to inherited PATH when `/bin/zsh` is absent | **M**: no shell, falls back; `%APPDATA%\npm`, `.cargo`, scoop not added |
| 4.4 | `user_path.rs:161-167`, `claude_code.rs:1128-1135` (`find_on_path`) | `.exe` appended on Windows; npm-installed CLIs are `.cmd` / `.ps1` shims | partial (`cfg!(windows)`) | – | **H**: `codex`, `opencode`, `kimi`, `grok`, `claude` installed through npm are not found; which vendors ship a real `.exe` is **U** (Claude ships `claude.exe`, VP 2026-10-27) |
| 4.5 | `kimi_code.rs:274`, `grok_build.rs:317`, `claude_code.rs:92`, `codex.rs:130`, `opencode.rs:243-258` | Fallback search dirs `/opt/homebrew/bin`, `/usr/local/bin`, `~/.npm-global/bin` | scattered | M: no Linuxbrew | **M**: none for `%LOCALAPPDATA%` etc. |
| 4.6 | `crates/juggler-adapter/src/system_tools.rs:21-47,57-64`; `crates/juggler-app/src/worktree.rs:72-84` | `git` only at `/opt/homebrew/bin`, `/usr/local/bin`, `/opt/local/bin`, `/usr/bin`; `GIT_MISSING` names Apple's Command Line Tools | partial seam (`cfg(unix)` `me()`, `developer_tools_installed()` true off-Mac) | L (copy) | **H**: no `C:\Program Files\Git\…` entry, so `git()` is always `None`; every worktree / git feature errors |
| 4.7 | `crates/juggler-adapters/src/quota_ingest.rs:168-290,1292-1318` | The Claude statusline wrapper is a `#!/bin/sh` script (`cat`, `date +%s`, `printf`, `sh -c`), `chmod 0755` under `cfg(unix)` | scattered | – | **H** (U): needs Claude Code to run statusline commands through Git Bash; no `.cmd` / PowerShell variant, so quota capture is likely absent |
| 4.8 | `crates/juggler-adapters/src/claude_environment.rs:952-961` | `make_link`: `os::unix::fs::symlink`, else `os::windows::fs::symlink_dir` | seam-real | – | L: needs Developer Mode or a privilege; the failure path is unchecked |
| 4.9 | `crates/juggler-adapters/src/transcript_tail.rs:87-96`; `crates/juggler-adapter/src/inventory.rs:307-326` | File identity by inode / device | cfg-unix; non-Unix returns `0` / lowercased canonical path | – | L |

### C5. Local IPC, file trust and private files

`std::os::unix` appears in about 40 non-test lines (a `grep` count, tests excluded by
path only). Most sit inside `cfg(unix)` items with a no-op elsewhere; the unguarded ones
are the compile blockers.

| # | file:line | Assumption | Guard | Lin | Win |
|---|---|---|---|---|---|
| 5.1 | `crates/juggler-app/src/features/agent_bridge/server.rs:50,54,77-104` | `tokio::net::UnixListener/UnixStream`, `FileTypeExt/MetadataExt/PermissionsExt`, `peer_cred().uid()`, `file_trust::me()` | **gap** (no cfg) | – | **B** |
| 5.2 | `features/agent_bridge/mod.rs:140,262-300` | `program()` uses `file_trust::me()`; `own_dir` uses `DirBuilderExt::mode(0o700)`; `write_private` uses `OpenOptionsExt::mode(0o600)` | **gap** | – | **B** |
| 5.3 | `features/runtime/launch.rs:80-99,124,169` | Driven-session socket: `harden`, `clear_stale`, `UnixStream::connect`, `file_trust::me()`; `Transport::Socket` | **gap** | – | **B** |
| 5.4 | `crates/juggler-adapter/src/file_trust.rs:9-33,66-140,142-156` | `me()`, `owner_of()`, `dir_is_ours()` exist only under `cfg(unix)`; non-Unix keeps only `trusted_executable` = `is_file`; `ADMIN_GROUPS` `[0,80]` (mac), `[0]` else | cfg-unix with holes: the missing non-Unix `me()` is what breaks 5.1-5.3 | L (AppImage runs from `/tmp/.mount_*`; `/tmp` is other-writable, so a bridge binary beside the exe may be rejected, **U**) | **B** (via 5.1-5.3); M: no owner / ACL check at all |
| 5.5 | `crates/juggler-agentbridge/src/client.rs:76-166` | `UnixStream` plus mode/uid checks | cfg-unix; non-Unix `exchange()` returns `Unreachable("the agent tools need macOS")`, the hook silently no-ops | – | **H**: the whole agent-bridge/hook channel is absent; copy says macOS |
| 5.6 | `features/agent_bridge/mod.rs:128-141` | `juggler-agent` sits beside the main exe; bundled as `externalBin` only in the macOS CI overlay | scattered | M: needs a per-target bundle | **M**: needs `juggler-agent.exe` |
| 5.7 | `crates/juggler-app/src/handover.rs:170-341` | Single-instance hand-over over a Unix socket (owner uid, 0700, `peer_cred`) | seam-stub: non-Unix `ask` → `NoneListening`, `serve` → `Ok(())`, `for_data_dir` → `None`; the Tauri single-instance plugin takes over. The file's own note says "a named pipe on Windows" | – | M |
| 5.8 | `crates/juggler-app/src/lib.rs:964-1000` | Old-app probe via `/tmp/<id>_si.sock` | seam-stub | – | L |
| 5.9 | `crates/juggler-link/src/identity.rs:245-291`, `registry.rs:466` | 0600 secret files, 0700 fleet dir, refuse group/other-readable | cfg-unix; non-Unix skips the checks | – | **M**: the Fleet identity key and registry rely on default ACLs, while the docs claim "owner only" |
| 5.10 | `crates/juggler-core/src/storage/mod.rs:667-676,737-746`, `transcript_index.rs:247-253,398`; `juggler-stats/src/store.rs:36-43`; `adapter/inventory.rs:419-428`; `adapters/claude_code.rs:1236-1262`; `features/handover/mod.rs:161-170`; `features/staging/mod.rs:170-179`; `commands.rs:1999-2007`; `fleet.rs:749-754`; `dev_bridge.rs:197-202`; `crash.rs:293-298` | Owner-only modes 0600 / 0700 | cfg-unix with a no-op elsewhere (the last three have no non-Unix branch) | – | **M**: defaults inherited from `%APPDATA%`; the isolation check at `commands.rs:1999` reports "owner-only (0700)" as passed without checking |
| 5.11 | `handover.rs:34-36,78-99` | Socket path in `temp_dir()`, max 100 bytes | scattered | – (`sun_path` is 108) | n/a |

### C6. Credential stores

Seam: trait `SecretStore` (`crates/juggler-account/src/secrets.rs:27`) with `KeychainStore`,
`VaultKeychain` (`vault.rs:292`), `MemoryStore`. `security-framework` is a macOS-only
dependency (`crates/juggler-account/Cargo.toml:24`).

| # | file:line | Assumption | Guard | Lin | Win |
|---|---|---|---|---|---|
| 6.1 | `secrets.rs:9,42-125` | macOS generic-password Keychain, service = edition bundle id (`ai.figurehead.juggler`, `.nightly`, `.dev`) | seam-stub: `get` → `None`, `set` → `Err(Unsupported)` ("needs the macOS Keychain"), `delete` no-op, `has` → `false` | **H** | **H** |
| 6.2 | `vault.rs:25-28,292-352` | Cloud-backup vault key in its own Keychain item | seam-stub | **H**: Cloud backup can't be turned on | **H** |
| 6.3 | `crates/juggler-app/src/edition.rs:93-121`, `lib.rs:637-640`, `features/service_accounts/mod.rs:40-67,188-190` | Per-edition services: device token, vault, `service-keys` | **seam-false**: `lib.rs:639` installs the Keychain store unconditionally, so `ServiceKeys::store()` returns `Ok` and the failure surfaces at `set` with the macOS wording. Accounts added "through another service" can't be added | **H** | **H** |
| 6.4 | `crates/juggler-app/src/figurehead.rs:77,110,395-401` | Device token in the Keychain | seam-stub; the account link fails off-Mac. The platform string is already generic (`{OS} {os} - {arch}`) | **H** | **H** |
| 6.5 | `claude_code.rs:293-295,785-796,1140-1141,1264-1318`; `system_tools.rs:12` | Probes Claude's `Claude Code-credentials[-<sha256(dir)[..8]>]` Keychain item by running `/usr/bin/security find-generic-password` (existence only, cached 30 s / 3 s) | seam-stub: off-Mac probes are skipped | L: Claude keeps the login in `<config dir>/.credentials.json` on Linux and Windows (VP, code.claude.com/docs/en/authentication, 2026-10-27), which `login_state` already reads first | L |
| 6.6 | `crates/juggler-adapters/src/codex.rs:267-269,316-331` | Probes Codex's `Codex Auth` Keychain item (`cli|<sha256[..16]>`) | seam-stub: off-Mac a missing `auth.json` reads `LoggedOut` | M: wrong only if the user set `cli_auth_credentials_store = "keyring"` (values `file`, `keyring`, `auto`, `ephemeral`, VP, learn.chatgpt.com/docs/auth, 2026-10-27; default `file` per `docs/research/codex.md`) | M (same, Credential Manager) |
| 6.7 | `crates/juggler-adapters/src/claude_environment.rs:123` | Copy: "the macOS Keychain locks it to that folder" | scattered | L | L (wrong text) |

### C7. Desktop-app detection and the host model

| # | file:line | Assumption | Guard | Lin | Win |
|---|---|---|---|---|---|
| 7.1 | `crates/juggler-adapter/src/live.rs:112-146,200-` (`HostApp`: `bundle` = `*.app` folder name, `bundle_id`, `tab_script` = AppleScript; `ctime_utc_secs` parses `ps -o lstart`) | Host apps are macOS bundles | scattered data model | **H**: no field for an exe name, `.desktop` id, or AUMID | **H** |
| 7.2 | `claude_live.rs:226-239` (`Claude.app`, `com.anthropic.claudefordesktop`); `codex_live.rs:70-83` (`ChatGPT.app`, `com.openai.codex`, link `codex://threads/{id}`); `features/conversations/hosts.json` (11 terminals/editors, all `*.app` + bundle ids; AppleScript tab scripts for Terminal and iTerm) | Bundles only | scattered | **H**: no process ever matches, so every session's host is `None` | **H** |
| 7.3 | `crates/juggler-app/src/manage.rs:620-632,912-917,942-948` | Editors found by `*.app` in `/Applications` and `~/Applications` | scattered | M: installed-editors list is empty, Open falls back to reveal | M |
| 7.4 | `commands.rs:14428`; `adapters/scan_signals.rs:205-220` (`Claude.app`, `ZCode.app` in `/Applications`) | Scan signal S10 | scattered | L | L |
| 7.5 | `crates/juggler-app/src/features/conversations/scan.rs:71` | `/usr/sbin/lsof -w -n -P -Fn -p` (2 s timeout) to find the transcript a process holds | scattered | M: `lsof` is usually `/usr/bin/lsof`, so the spawn fails and held-session detection is absent. (`/proc/<pid>/fd` is the Linux way) | M: absent |
| 7.6 | `features/app_icons/bundle.rs:15-21,136,155-221` | Icons read from `Contents/Info.plist`, `*.icns`, `Assets.car` under `/Applications`, `/System/Applications` | scattered folders; the reader is behind 12.1 | L | L |
| 7.7 | `crates/juggler-app/src/machine_tools.rs`, `machine_memory.rs:36-60` | Bundle names for "This Mac" | scattered | L | L |

### C8. Go to it, focus, permissions, URL schemes

Seams: trait `Front` (`activation.rs`), trait `Desktop: Front` (`features/conversations/goto.rs:30`),
trait `ScriptRunner` (`focus.rs`), module `os_access::platform`.

| # | file:line | Assumption | Guard | Lin | Win |
|---|---|---|---|---|---|
| 8.1 | `crates/juggler-app/src/activation.rs:132-232` | NSRunningApplication cooperative activation | seam-stub: non-macOS `Request::Refused`, `frontmost` `None`. The outcome copy (`focus.rs:40-56`) says "macOS didn't bring the app forward … click it in the Dock" | **H** | **H** |
| 8.2 | `crates/juggler-app/src/features/conversations/goto.rs:379-446` (`MacDesktop`: `open_link` = `/usr/bin/open`, `osascript` = `/usr/bin/osascript`, `tty` = `/bin/ps -o tty=` requiring `ttys<N>`); wired unconditionally at `features/conversations/commands.rs:306,325` | macOS tools by absolute path | **gap** (no non-Mac `Desktop` implementation) | **H**: `open` / `osascript` missing; `ps` tty is `pts/N`, so `None`. Fails quietly | **H** |
| 8.3 | `focus.rs:96-104,334-341,775-778` | System Events / AX scripting; screen lock via `ioreg` | seam-stub: `focus()` returns `OUTCOME_UNSUPPORTED` ("Window focus unavailable on this platform") off-Mac; `screen_locked` → `Err`, so Fleet remote focus rejects with `lock_unknown` | M | M |
| 8.4 | `crates/juggler-app/src/os_access.rs:57-386` | Accessibility, Automation, Screen Recording prompts | seam-stub: `entries()` → empty (first-run step skipped), `exact_windows_allowed` → `false`, unsupported ids rejected. **The cleanest seam in the repo** | L | L |
| 8.5 | `crates/juggler-app/src/features/processes/commands.rs:473,504` | `/usr/bin/open <folder>`; `/usr/bin/open -b com.apple.ActivityMonitor` | scattered | M: spawn error swallowed, returns `Ok(false)` | M (same). `tauri_plugin_opener` is already used for the other reveal paths (`manage.rs:1082-1096`, `commands.rs:1838,2625,3584`) |
| 8.6 | `crates/juggler-app/src/newer_data.rs:610-621` | `open` / `explorer` / `xdg-open` by absolute path | **seam-real** (`cfg!` three-way; the model to copy). `XDG_OPEN` is a fixed `/usr/bin/xdg-open` | – | – |
| 8.7 | `crates/juggler-app/Cargo.toml`, `tauri.conf.json`, `Info.plist` | AI Juggler registers **no** URL scheme and handles no deep link | n/a | – | – (a port needs nothing here; the `codex://` open path is 8.2) |
| 8.8 | `crates/juggler-app/src/lib.rs:733-740,880-897` | `tauri_plugin_single_instance` | portable plugin | – | – (a second launch relies on the plugin alone, see 5.7) |
| 8.9 | `lib.rs:723-731,1063-1079` | `tauri-plugin-global-shortcut` hotkey | portable plugin | L: unsupported on native Wayland (U; Claude's own Linux note says Quick Entry "requires your desktop environment's GlobalShortcuts portal" on Wayland, VP 2026-10-27) | – |
| 8.10 | `crates/juggler-app/src/quit.rs:295-314` | `app_menu` replaces the last item of the first submenu with Quit `CmdOrCtrl+Q` ("the app submenu comes first") | scattered | L (the first submenu is File) | L |

### C9. Notifications

| # | file:line | Assumption | Guard | Lin | Win |
|---|---|---|---|---|---|
| 9.1 | `crates/juggler-app/src/features/notifications/mod.rs:360-390`; `lib.rs:723`; `Cargo.toml:70` (`tauri-plugin-notification = "2.6"`, "first that reports a click on macOS") | Plugin shows the toast; a click comes back through `on_action` | portable plugin | M (U: whether `on_action` fires on desktop Linux) | M (U); the plugin's docs say Windows toasts work only for installed apps, dev builds show the PowerShell name and icon (VP, v2.tauri.app/plugin/notification, 2026-10-27) |

### C10. Login item, keep-awake, machine signals

| # | file:line | Assumption | Guard | Lin | Win |
|---|---|---|---|---|---|
| 10.1 | `crates/juggler-login-item/src/lib.rs:40-108` | `SMAppService` | seam-stub: `status()` = `Unavailable`; `set()` = `Err("Opening at login is only on macOS.")` | M | M |
| 10.2 | `crates/juggler-supervisor/src/awake.rs:43,99-179` | `NSProcessInfo performActivity` (App Nap and idle-sleep) | seam-stub: `Held` is an empty struct; nothing is claimed. Overnight runs can sleep | **H** (core overnight feature) | **H** |
| 10.3 | `crates/juggler-supervisor/src/machine.rs:534-552,614-625`; `app/machine_alerts.rs:261-334` | Memory pressure level from `kern.memorystatus_vm_pressure_level`; copy "macOS reports …" | seam-stub: `pressure()` → `None`; no pressure alert | M (`/proc/pressure/memory`, I) | M |
| 10.4 | `machine.rs:490-528,581-597`; `juggler-vm-stats/src/lib.rs:34-84` | "In use" = app + wired + compressed (`host_statistics64`, sysctl) | seam-real: the trait default is total minus available, tagged `MemoryBasis::TotalMinusAvailable` (`machine.rs:471-476`) | L | L |
| 10.5 | `machine.rs:554-579` | Low Power Mode via IOKit / NSProcessInfo | seam-stub → `None`; `features/app/awake.rs:79-83` treats `None` as on power | M | M |
| 10.6 | `machine.rs:599-611` | Thermal state via NSProcessInfo | seam-stub → `None` | M | M |
| 10.7 | `machine.rs:883-896` | `mount_point == "/"` and `starts_with("/Volumes/")` | scattered | – | L (U: `C:\` and `\\?\` prefixes) |
| 10.8 | `crates/juggler-app/src/machine_memory.rs:99`, `machine_alerts.rs:547`, `commands.rs:9961` | Display units: decimal GB in some places, GiB in another | scattered | L | L (decimal matches Finder; Windows Explorer shows binary sizes labelled GB) |

### C11. Ports

| # | file:line | Assumption | Guard | Lin | Win |
|---|---|---|---|---|---|
| 11.1 | `crates/juggler-supervisor/src/ports.rs:64-110`, `lib.rs:48-51`, `live_stats.rs:865-876`, `commands.rs:7707-7714`; `Cargo.toml:28-44` | `NetstatProbe` (netstat2 0.11), gated to Unix | seam-stub: non-Unix `UnsupportedProbe` → `ports_degraded` = "ports unknown on this platform" | – (netstat2's Linux coverage not verified, **U**) | **H**: the port feature is absent. Whether netstat2 supports Windows is **U**; widening the `cfg` and the dependency gate would be the fix if so |

### C12. Icons, trash, window chrome

| # | file:line | Assumption | Guard | Lin | Win |
|---|---|---|---|---|---|
| 12.1 | `crates/juggler-app/src/features/app_icons/render.rs:97-214` | `NSWorkspace iconForFile` | seam-stub: `icon_png` → `None`, the UI draws letters | L | L |
| 12.2 | `crates/juggler-app/src/manage.rs:490-515,333` | `move_to_trash` via NSFileManager; the `trash` crate is a macOS-only dependency (`Cargo.toml:127`) | seam-stub: `Err(Unsupported, "moving to the Trash isn't available on this system yet")`. Removing an account with `CopyFolder::Trash` fails before anything is forgotten | M | M (the crate itself supports Windows and Linux, I) |
| 12.3 | `crates/juggler-app/tauri.conf.json:20-21`; `ui/src/App.tsx:695,1221` | `titleBarStyle: "Overlay"`, `hiddenTitle`, `data-tauri-drag-region`; the UI draws its own header | macOS-only keys, ignored elsewhere | M: a native title bar above the UI's own header | M |
| 12.4 | `crates/juggler-app/icons/` (has `icon.ico`, `icon.png`, no `icon.icns`); `scripts/app_icon.swift`, `scripts/edition_icon.swift`; `justfile:168,249` | Icon pipeline is Swift / AppKit | macOS tooling | M: the Dev edition icon needs a replacement | M |

### C13. Screen capture and the Swift bridge

| # | file:line | Assumption | Guard | Lin | Win |
|---|---|---|---|---|---|
| 13.1 | `crates/juggler-scap-init/src/lib.rs:20` (`#![cfg(target_os="macos")]`, empty crate elsewhere); `crates/juggler-app/src/preview.rs:62-70,330-445`; `commands.rs:3465-3470`; `Cargo.toml:112-146` | ScreenCaptureKit window mirror; macOS-only deps (`screencapturekit`, `objc2*`) | seam-stub: `UnsupportedSource` → `CaptureError::Unsupported` | L | L. Also dead today on every OS: no real adapter declares `DESKTOP_APP` (`os_access.rs:196-206`) |
| 13.2 | `scripts/ensure_swift_runtime.sh`, `justfile:7-9` | A Swift link shim for the Command Line Tools only Mac | macOS tooling | L | L |

### C14. Updater, signing and packaging

| # | file:line | Assumption | Guard | Lin | Win |
|---|---|---|---|---|---|
| 14.1 | `crates/juggler-app/tauri.conf.json:32-37` (`bundle.active=false`; only `macOS.entitlements`); `tauri.dev.conf.json`; `tauri.nightly.conf.json`; the real bundle config is a CI overlay in `.github/workflows/build-macos.yml:~196-215` (`targets:["app"]`, `externalBin`, macOS min 14.0, `createUpdaterArtifacts:false`) | Bundles are produced by a macOS-only CI overlay | macOS only | **H**: no `bundle.targets` for deb / AppImage / rpm, no `externalBin` triple for `juggler-agent` | **H**: none for nsis / msi, `webviewInstallMode` |
| 14.2 | `crates/juggler-app/src/updates.rs:35-43,116-117,218-220,1038-1043,1324`; `.github/workflows/nightly.yml:148-160`; `promote.yml:198-216` | Feed `…/<channel>-channel/latest.json`; `platforms` is only `darwin-aarch64` and `darwin-x86_64`, both pointing at `…-universal.app.tar.gz` | macOS only | **H**: no `linux-*` keys | **H**: no `windows-*` keys |
| 14.3 | `crates/juggler-app/Cargo.toml:73-78`; `docs/RELEASING.md:202-204` | `tauri-plugin-updater` built **without** its default `zip` feature (open advisories in the `zip` version); the comment says to re-enable when Windows releases start | deliberate | – | **H**: Windows installers cannot be extracted, so an update would fail at install |
| 14.4 | `scripts/sign_macos.sh` (codesign, notarytool, `hdiutil`, `.app.tar.gz` + `.sig`), `scripts/apple_retry.py`, `build-macos.yml:1-120`; `entitlements.plist`; `Info.plist` | Apple signing and notarization; hardened runtime, Local Network / Bonjour for Fleet | macOS only | M (Linux packages are unsigned or repo-signed; Claude does apt with a GPG key, VP 2026-10-27) | **H**: no Authenticode path. Fleet's mDNS will hit the Windows firewall prompt (U) |
| 14.5 | `.github/workflows/{build-macos,nightly,promote,release-rehearsal}.yml` | All signing and packaging jobs run on macOS | macOS only | **H** | **H** |
| 14.6 | `crates/juggler-app/src/main.rs:5`; `build.rs`; `capabilities/*.json` | `windows_subsystem="windows"` in release; portable build script and capabilities | already Windows-aware | – | – |
| 14.7 | `crates/juggler-app/Cargo.toml:64-66` | `rfd` built with `gtk3` and `common-controls-v6` | already both | – | – |

### C15. CI and developer tooling

| # | file:line | Assumption | Guard | Lin | Win |
|---|---|---|---|---|---|
| 15.1 | `.github/workflows/ci.yml:28` (verify, ubuntu: fmt, clippy ×3, `cargo test --workspace`, deny, coverage); `:344` (build-macos: tests incl. collateral and machine signals); `:396` (build-linux: `cargo build --workspace`); `:420` (build-windows: `cargo build --workspace`, **no test run**) | Tests run only on Linux and macOS; Windows is compile-only | partial | – (tests already run in `verify`) | **M**: Windows runtime behaviour is never exercised. Green on `main` (run 40488659421, 2026-10-27) but not seen on `release/0.2` |
| 15.2 | `scripts/install_tauri_cli.sh:15-20` | Only `cargo-tauri-{aarch64,x86_64}-apple-darwin.zip`; exits otherwise | macOS only | M | M |
| 15.3 | `justfile`, `scripts/*.sh` | bash plus swift | bash only | – | **M**: a Windows contributor needs Git Bash / WSL |
| 15.4 | `rust-toolchain.toml` (1.99.0, no `targets`); `deny.toml` (`all-features = true`) | No target list; cargo-deny already sees the Linux stack | – | – | L: Windows-only crates untested |

### C16. UI copy and shortcuts

| # | where | Assumption | Lin | Win |
|---|---|---|---|---|
| 16.1 | `ui/src/lib/shortcuts.ts` (47 lines with ⌘ ⌥ ⇧), `hotkey.ts`, `rail-view.ts`, `app-keys.ts`, `native-menu.ts`; `components/ShortcutSheet.tsx:41-44` shows `entry.keys` raw | Matchers already accept `metaKey || ctrlKey`, terminal tools use Ctrl+Shift off-Mac (`TerminalToolbar.tsx:18-19`, `navigator.platform`), but visible shortcut text is ⌘/⌥ glyphs | M | M |
| 16.2 | `ui/src` (non-test, non-generated line counts): "macOS" 48, "This Mac" 73 (also the screen name `ThisMacScreen`), "your Mac" 13, "Finder" 20, "System Settings" 15, "Keychain" 9, "Activity Monitor" 9, "Trash" 8, "Accessibility" 10, "Screen Recording" 4, "Dock" 3. Top files: `lib/machine.ts`, `components/this-mac-memory.tsx`, `lib/osAccess.ts`, `lib/machine-memory.ts`, `components/NotificationSettings.tsx` | Mac vocabulary hard-coded | M | M |
| 16.3 | Rust-side user strings: `manage.rs` (36 lines), `link.rs` (32), `commands.rs` (19), `figurehead.rs`, `backup.rs`, `machine_alerts.rs:261-334`, `processes/query.rs:256`, `focus.rs:40-56`, `secrets.rs:18` | Same vocabulary; the honest-stub messages themselves say "macOS" | M | M |
| 16.4 | `ui/index.html:18`, `ui/src/styles.css:44` | System font stack has a Segoe UI fallback | – | – |
| 16.5 | `crates/juggler-app/src/lib.rs` (`not_answering_words`), `newer_data.rs:610-621` | Already branch on `cfg!(macos)/cfg!(windows)/else` | – | – (the model to copy for strings) |

**Totals (approximate, counted by hand).** About 115 inventory rows. Blockers: 4 rows
(all Windows, one root cause: Unix IPC and trust code without a seam). Highs on Linux
are all missing *features* (credentials, Go to it, keep-awake, host detection, packaging,
updater feed, `lsof`); none is a wrong answer. The Windows highs include the wrong-answer
or false-success rows 2.5, 3.1-3.6, 4.2, 4.6 and 5.5.

---

## 3. Seams: what to put each concern behind

The principle of the architecture standard applies to platforms as well as tools: a
module with one trait, one implementation per OS, and a `NotSupported` that says so. Most
of this already exists under another name (the existing seam is cited). A single new
crate, **`juggler-platform`**, is proposed to hold the traits that have no home yet, so
"where does the Windows version go?" has one answer. The crate must stay free of tool
and product knowledge; adapters keep tool-specific paths (they *call* it).

Note on `unsafe_code = "forbid"` (workspace `Cargo.toml`): it applies to the workspace
crates, not to their dependencies. Windows Job Objects, `SetThreadExecutionState` and
Credential Manager each need `unsafe` somewhere. Prefer a vetted wrapper crate (a
dependency) before opening a fourth carve-out beside `juggler-scap-init`,
`juggler-login-item` and `juggler-vm-stats` (D-17, D-26, D-27). Which wrapper is a
choice for the implementing session (I: candidates are `keyring`, `win32job`, `trash`,
`tauri-plugin-autostart`, `interprocess`; none was evaluated here).

| Concern | Seam (trait / module) | macOS | Linux | Windows | What Tauri / crates already give |
|---|---|---|---|---|---|
| **C1** Paths | `Home` + `AppDirs` in `juggler-platform`: one resolver (`HOME` on Unix, `USERPROFILE` on Windows, no `dirs::home_dir`), `data_dir` = local app data. Adapters own `tool_home(tool) -> ToolHome { path, env_override }` | as today | as today, honour `XDG_*` for OpenCode | `%LOCALAPPDATA%` for the DB; `%USERPROFILE%` homes | `dirs` for known folders (VP: Windows data dir is Roaming); Tauri `app_data_dir` / `app_local_data_dir` |
| **C2** Executable identity | `ExeId` value type: `from_path` / `from_name`, splits on both separators, strips `.exe` / `.cmd`, case rules per OS, answers `app_bundle()` (macOS `.app`), `is_helper_of`. Replaces `rsplit('/')` and `file_name == "claude"` everywhere | `.app` bundles | plain name, 15-char `comm` aware | `.exe`, case-insensitive, `\\?\` stripped (cf. Grok's use of `dunce`, VP) | `sysinfo` |
| **C3** Stop | Extend `signal_pid` into `ProcessControl { terminate_tree, kill_tree, is_alive, members }` and make **`is_alive` fallible** (`Alive / Gone / Unknown`) so a stub can never read as "gone". Unix = process groups + `nix` (today); Windows = Job Object (assign at spawn, `TerminateJobObject`) | today | today | Job Object | `portable-pty` kills the root only; `sysinfo` can enumerate children |
| **C4** Spawn / env | `LaunchEnv`: a per-OS allowlist (Windows adds `USERPROFILE SystemRoot APPDATA LOCALAPPDATA PATHEXT TEMP TMP ProgramFiles*`; Linux adds `XDG_*`, `DISPLAY`, `WAYLAND_DISPLAY`) and a `ToolLocator` (PATH plus `PATHEXT`, `.cmd` shim resolution, known dirs per OS, `git` places per OS) | today | extend | new | `portable-pty` (ConPTY, I); `which`-style lookup is a candidate crate (I) |
| **C5** IPC and trust | `LocalIpc` (listener, connector, `peer_identity`) with Unix socket and **Windows named pipe** implementations, and `PrivateFs` (`create_private_dir`, `write_private`, `is_ours`, `restrict_to_owner`) replacing the the scattered `std::os::unix` sites. `file_trust::me()` becomes `file_trust::current_user()` returning a type that exists everywhere | Unix | Unix | named pipe (the `handover.rs` note already says so); ACL helpers | `tauri-plugin-single-instance` already does hand-over on all three; `tokio` has `windows::named_pipe` (I) |
| **C6** Credentials | Keep `SecretStore`. Add one real implementation backed by the OS store (Keychain / Credential Manager / Secret Service) and make `ServiceKeys::store()` honest (**seam-false** 6.3). Probes of *vendor* credentials stay in adapters and read files first | `security-framework` (today) | Secret Service, falling back to an encrypted file under the data dir | Credential Manager | `keyring` crate lists macOS, Windows and Linux (Secret Service, keyutils) back ends as optional dependencies (VP, docs.rs/keyring, 2026-10-27; feature names not verified, size limits **U**) |
| **C7** Host apps | `HostApp` becomes an enum of `HostId { MacBundle{..}, WinExe{..}, LinuxDesktop{..} }` plus `InstalledApps` (list by name / id), `RunningApp` and an `AppIcon` provider. `hosts.json` gains per-OS entries | bundles | `.desktop` files and `Exec=`; Flatpak / Snap ids | exe names, Start Menu, AUMID | no Tauri API; per-OS code needed |
| **C8** Go to it | `Desktop` trait gets a real implementation per OS with *capability reporting*: `can_focus_window`, `can_open_link`, `can_find_tty`. Linux/Wayland has no cross-app focus API, so the honest answer there is "open", not "go" | `MacDesktop` | `xdg-open` / `gio open` for links and folders; X11 only: `wmctrl`-style focus (I) | `explorer` / `start`; `SetForegroundWindow` is restricted (I) | `tauri-plugin-opener` for open / reveal (already used) |
| **C9** Notifications | Keep the plugin; add a `Notifier` seam only if click routing differs (the click path is U on Linux and Windows) | plugin | plugin | plugin (installed builds only, VP) | `tauri-plugin-notification` |
| **C10** Awake / login / machine | `KeepAwake` (exists as `awake::imp`), `LoginItem` (exists), `MachineSignals` (exists as `OsReadings`). Fill in: Linux `systemd-logind` / `org.freedesktop.ScreenSaver` inhibit; Windows `SetThreadExecutionState`; autostart `.desktop` / Run key; PSI for pressure | today | D-Bus inhibit; `~/.config/autostart` | `SetThreadExecutionState`; Run key | `tauri-plugin-autostart` covers all three OSes in its supported-platforms table, though the page names only the macOS mechanism (VP, v2.tauri.app/plugin/autostart, 2026-10-27); `sysinfo` has memory, CPU, components |
| **C11** Ports / open files | `PortProbe` (exists) widened to Windows if `netstat2` supports it (U); `OpenFiles` trait replacing the `lsof` spawn (`/proc/<pid>/fd` on Linux, Restart Manager / handle enumeration on Windows, I) | `lsof` / libproc | `/proc` | handle API (hard; may stay "unknown") | `netstat2` |
| **C12** Reveal / trash / chrome | `Reveal` through `tauri-plugin-opener` everywhere; `Trash` through the `trash` crate (supports Windows and Linux, I) instead of an NSFileManager stub; window chrome through per-OS `tauri.conf` overlays | | | | `tauri-plugin-opener`, `trash` |
| **C13** Capture | `CaptureSource` (exists). Leave `UnsupportedSource` until a real adapter declares `DESKTOP_APP` | | | | n/a |
| **C14** Update / sign / package | One `release/platforms.json` listing target triples, bundle formats, signing step and updater key (`linux-x86_64`, `windows-x86_64`, `darwin-*`); the workflows read it instead of hard-coding macOS | dmg / `.app.tar.gz` | AppImage (the Tauri updater format) and `.deb`; Anthropic ships Claude Desktop as an apt repository (VP, 2026-10-27), a model for a later channel | NSIS or MSI, Authenticode, `zip` feature on | Tauri 2 bundles: Windows installer and Store; Linux deb, rpm, AppImage, Snap, Flatpak, AUR; macOS app and dmg (VP, v2.tauri.app/distribute, 2026-10-27) |
| **C15** CI / dev | A `platform` job matrix: `cargo check --workspace --all-targets` on Windows and Linux (not only `build`), and `cargo test` on Windows for the crates that claim support | | | | |
| **C16** Copy | A small `Platform` module in the UI (`modifierGlyph`, `fileManagerName`, `settingsName`, `credentialStoreName`) and in Rust the `cfg!` three-way helper already used by `not_answering_words` / `show_folder` | | | | |

### What is already cross-platform and should simply be used

- **`sysinfo`** supports Linux, macOS and Windows (VP, docs.rs/sysinfo, 2026-10-27). It is already the one process source.
- **`portable-pty`**, **`notify`**, **`dirs`**, **`rfd`**, **`tauri-plugin-{opener,dialog,notification,global-shortcut,single-instance,updater}`** are all in the tree and portable. `tauri-plugin-deep-link` is not needed (AI Juggler registers no scheme, row 8.7). If it is added later, on Linux and Windows links arrive as command-line arguments and the `deep-link` feature of the single-instance plugin forwards them (VP, v2.tauri.app/plugin/deep-linking, 2026-10-27).
- **`nix`** is Unix-only and already gated.

---

## 4. What each supported tool stores, per OS

Date read for new claims: 2026-10-27. "Carried" means taken from the repo's own research
(`docs/research/*.md`, dated 2026-10-21) and not re-read today.

| | macOS (today) | Linux | Windows |
|---|---|---|---|
| **Claude Code CLI** | `~/.claude`, `~/.claude.json`; login in Keychain, fallback `~/.claude/.credentials.json` | Same dot-folder; login in `~/.claude/.credentials.json`, mode 0600. **VP**, code.claude.com/docs/en/authentication, 2026-10-27 | `%USERPROFILE%\.claude`, `%USERPROFILE%\.claude.json`; login in `%USERPROFILE%\.claude\.credentials.json`, inheriting the profile directory's ACL. **VP**, same page and `claude-directory` page, 2026-10-27 |
| …config-dir override | `CLAUDE_CONFIG_DIR` moves `.credentials.json` and keys the Keychain entry to that directory (**VP**) | same | same |
| …binary | `~/.local/bin/claude` → `~/.local/share/claude/versions/` | same; apt, dnf, apk also publish signed repos | `%USERPROFILE%\.local\bin\claude.exe` and `%USERPROFILE%\.local\share\claude`; also WinGet `Anthropic.ClaudeCode` (**VP**, code.claude.com/docs/en/setup, 2026-10-27) |
| …platforms | macOS 13+ | Ubuntu 20.04+, Debian 10+, Alpine 3.19+ | Windows 10 1809+ native; Git Bash optional, else PowerShell tool; WSL 1 / 2 (sandboxing only on WSL 2) (**VP**) |
| **Claude desktop app** | `~/Library/Application Support/Claude`; `claude-code-sessions/…/local_<id>.json` (read by `claude_live.rs`) | **Beta, Debian / Ubuntu 22.04+ only**, apt repo, no self-update (**VP**, code.claude.com/docs/en/desktop-linux, 2026-10-27). Login kept in the desktop keyring (GNOME Keyring / KDE Wallet) (**VP**). Data dir: sources disagree (`~/.config/Claude` vs `~/.config/claude-desktop`) (**VS**, web search 2026-10-27) → **U** | x64 and ARM64 (**VP**). Data dir `%APPDATA%\Claude` (**VS**, 2026-10-27) → treat as **U** until a lab confirms; the `claude-code-sessions` layout there is **U**. The Code tab can run sessions **inside a WSL 2 distribution** (Linux paths, `\\wsl.localhost\…`) (**VP**, code.claude.com/docs/en/desktop-wsl, 2026-10-27) |
| **Codex CLI** | `CODEX_HOME`, default `~/.codex`; `auth.json` default (`cli_auth_credentials_store` = `file`; per research) or Keychain item `Codex Auth` | Same; `keyring` store means the OS credential store (**VP**, learn.chatgpt.com/docs/auth, 2026-10-27: values `file`, `keyring`, `auto`, `ephemeral`; default not stated there, `file` per carried research) | `CODEX_HOME` example `C:\Users\alice\.codex`; Windows sandbox logs in `CODEX_HOME\.sandbox\` and secrets in `CODEX_HOME\.sandbox-secrets\` (**VP**, learn.chatgpt.com/docs/windows/windows-sandbox, 2026-10-27). Native Windows or WSL (**VP**) |
| **Codex desktop** (in the ChatGPT app) | `/Applications/ChatGPT.app`, bundle id `com.openai.codex`, `codex://threads/{id}` | The docs navigation lists a "Linux desktop app" page; its content was **not read** (the URL I tried returned 404) → **U** | Native app that "can run natively in PowerShell with a Windows sandbox", WSL optional (**VP**, 2026-10-27). Process / exe name, Store identity and data dir **U** |
| **OpenCode CLI** | Data `~/.local/share/opencode` (`opencode.db`, `auth.json`), config `~/.config/opencode/`; `XDG_*` honoured (carried, VP; `opencode.rs:70-74` consts) | Same; managed config `/etc/opencode/` | Data and logs `%USERPROFILE%\.local\share\opencode` (**VP**, opencode.ai/docs/troubleshooting, 2026-10-27); user config path not stated → **U** (I: `%USERPROFILE%\.config\opencode`); managed config `%ProgramData%\opencode` (**VP**); "can run directly on Windows" but WSL recommended, and in WSL data is `~/.local/share/opencode` inside the distro (**VP**, opencode.ai/docs/windows-wsl) |
| …state / service | `service.json` in `$XDG_STATE_HOME/opencode` (v2) | `~/.local/state/opencode` | **U** |
| **OpenCode desktop** | Electron; `/Applications/OpenCode.app`, `ai.opencode.desktop[.beta/.dev]`; shares the CLI's data dir (carried, VP) | `.deb` / `.rpm` downloads (carried, VP, https://opencode.ai/download) | x64 / ARM64 downloads (carried, VP); install path, exe name **U** |
| **Kimi Code CLI** | `~/.kimi-code` (`KIMI_CODE_HOME`); OAuth in `credentials/<name>.json`, mode 0600 (carried, VP) | `/home/<name>/.kimi-code` (**VP**, moonshotai.github.io/kimi-code data-locations, 2026-10-27) | `C:\Users\<name>\.kimi-code` (**VP**, same page). `.agents` stays under the real OS home (**VP**). Credential file mode 0600 has no Windows meaning → **U** how it is protected |
| **Kimi Code desktop** | UI state `~/Library/Application Support/kimi-code-app/`, scheme `kimi-code://` (carried, VP) | Exists (macOS, Windows, Linux, carried VP `docs/research/kimi.md:237`); data dir **U** | same, data dir **U** |
| **Grok Build CLI** | `~/.grok` (`GROK_HOME`), `auth.json` mode 0600, no Keychain use found (carried, VP; NE-6 in `docs/research/grok.md`) | same | Prebuilt binaries for Windows (PowerShell, Git Bash, MSYS2); WSL uses the Linux binary (carried VP); `GROK_HOME` "verbatim when non-empty, else `<home>/.grok`" with `home` from `std::env::home_dir` (USERPROFILE on Windows) and no APPDATA handling (**VP**, xai-org/grok-build `xai-dirs/src/lib.rs`, 2026-10-27) |
| **Grok desktop** | none (Grok Bot is a separate cloud-agent product) | none | none |

**Consequences for AI Juggler:**

- The four dot-folder tools need no per-OS path logic beyond the home resolver (C1): the
  existing `~/.claude`, `~/.codex`, `~/.kimi-code`, `~/.grok` constants are right on all three OSes.
- Per-account isolation by directory (the account model) is **easier** off-Mac: there is no
  Keychain entry keyed to the folder path, because the login is a file in the folder
  (6.5). The macOS-only hash logic (`namespaced_service`) can stay in the macOS module.
- The big unknown is the **desktop apps**. Their process names, install locations and data
  dirs on Windows and Linux are largely U. A lab pass on a Windows and a Linux machine
  (read-only, sacrificial profile) should come before any adapter work (5.2, step 8).
- **WSL is a new concept.** A Windows desktop app and a Windows AI Juggler can host
  sessions whose processes and files live in a WSL 2 distribution. `sysinfo` on Windows
  does not list those processes. Decide early whether WSL sessions are in scope for the
  first Windows build (recommended: no, state it plainly, track the Windows-side
  processes only; revisit with `wsl.exe` after the first release).

---

## 5. Ranked plan

The ordering rule: **first stop new code adding macOS-only assumptions, then make it
compile, then make the wrong answers honest, then fill features.** Steps 1-4 touch no
user-visible behaviour on macOS.

### 5.1 Stop the bleeding (do first, small)

1. **Find out what is red.** Run the existing `build-windows` and `build-linux` jobs on a
   `release/0.2` PR (or `cargo check --workspace --target x86_64-pc-windows-msvc` locally).
   Everything in section 2 marked **B** is by reading only. *(Half a day.)*
2. **Add a platform guard lint, in the style of `scripts/tool_literal_lint.py`** (counts per
   file against a shrink-only allowlist, ADR-0019 §5) **and `tests/no_bare_tools.rs`**
   (which already enforces absolute tool paths): outside `crates/juggler-platform` and the existing seam
   modules, fail CI on `std::os::unix`, `UnixListener`, a literal `/usr/bin/`,
   `/Applications/`, `rsplit('/')` on a path, `.app` string matching, and the word
   "macOS" / "Mac" in a user-facing string literal. Its allowlist starts as the
   current offenders (this document's inventory) and only shrinks, as the tool-literal
   and plain-words lints do.
3. **Promote the Windows job.** `cargo check --workspace --all-targets` (compiles tests too)
   on Windows and Linux; make both required for `release/*`. Today a green Windows
   `build` does not compile tests and ran on `main` only.
4. **Fix the false-success** (3.1-3.4, 3.6): make `signal_pid`'s non-Unix branch return a
   tri-state (`Alive / Gone / Unknown`) so no stub can say "stopped". This is a ten-line
   change that removes the only place where a Windows build could claim something false.
   Until the Job Object lands, Windows `stop` should return *"could not stop the whole
   group"* instead of success.

### 5.2 Seams, in order of how much each unlocks

| Step | Seam | Why this rank | Removes |
|---|---|---|---|
| 1 | **`juggler-platform` crate + `PrivateFs` + `LocalIpc`** (C5) | Unblocks the Windows compile; the scattered `std::os::unix` sites; new features keep adding more (0.2 added three modules) | B 5.1-5.4, 5.7, 5.9, 5.10 |
| 2 | **`ExeId`** (C2) | One value type replaces ~20 hand-rolled `rsplit('/')` / `== "claude"` checks and the `.app` string matching; it is also what Linux needs for 15-character `comm` names | 2.1-2.6, 3.9, 7.2 (partly) |
| 3 | **`Home` / `AppDirs` / `ToolHome`** (C1) | Three resolvers today; the data-dir location (`data_local_dir`) must be decided before any Windows build ships because moving it later is a migration | 1.1-1.4, 1.10, 4.3 |
| 4 | **`ProcessControl` with Job Object** (C3) | The only seam where Windows has no implementation at all | 3.1-3.6 |
| 5 | **`LaunchEnv` + `ToolLocator`** (C4) | Without `USERPROFILE`, `SystemRoot`, `PATHEXT` and `.cmd` shims, no tool is found or probed on Windows | 4.2-4.6 |
| 6 | **`SecretStore` real implementation** (C6) | Keeps accounts through another service, Figurehead link and Cloud backup from failing with a macOS message; makes `ServiceKeys::store()` honest | 6.1-6.4 |
| 7 | **`Desktop` / `HostApp` model** (C7, C8) | Largest design change (the host model is a macOS bundle today); needs the lab results first | 7.1-7.3, 8.1-8.2 |
| 8 | **`KeepAwake` / `LoginItem` / `MachineSignals`** fill-ins (C10) | Seams exist; the work is per-OS implementations. Keep-awake is the overnight-run feature, so it is the first to fill | 10.1-10.6 |
| 9 | **Release descriptor** (C14) | Needed once there is anything to ship; decides updater keys, bundle formats, signing | 14.1-14.5 |
| 10 | **Copy / shortcut layer** (C16) | Large but mechanical; do it per screen as screens are touched, enforced by the guard test | 16.1-16.3 |

### 5.3 Minimum for a first Linux build

What already works (read, not run): data dir, Unix sockets and trust, process groups and
signals, PTY, ports, the dot-folder tools and `~/.claude/.credentials.json`, single
instance, CI build plus tests.

Required:
1. `build-linux` stays green on `release/0.2` with tests compiled (5.1 steps 1 and 3).
2. **Packaging**: `bundle.targets` for AppImage and `.deb`, `juggler-agent` as an
   `externalBin` for the Linux triple, an AppImage-safe `trusted_executable` (5.4 and 5.6;
   `/tmp/.mount_*` is **U**), a `linux-x86_64` entry in the updater feed (Tauri's updater
   handles AppImage; `.deb` would update through a package repository, as Claude Desktop does).
3. **Credentials** (6.1-6.4): a Secret Service back end with an explicit fallback, or the
   honest message "no keyring available" instead of the macOS text.
4. **Launch environment** (4.2): pass `XDG_*`, `DISPLAY`, `WAYLAND_DISPLAY`.
5. **Honest degraded features**: Go to it / focus says "open" on Linux (8.2), the "This
   Mac" copy becomes "This computer" (16.2), `lsof` becomes `/proc` or "unknown" (7.5).
6. **Keep-awake** (10.2) via D-Bus inhibit, or the UI states that this machine may sleep.
7. **Autostart** (10.1) via `.desktop`, or hidden.
8. Window chrome (12.3): a native title bar or `decorations: false` with the UI header.

Deferred without harm: Claude desktop detection (Linux beta, Debian only, data dir
**U**), Codex Linux desktop (**U**), memory-pressure alerts, thermal, power, screen-lock,
icons (letters are drawn), Fleet's mDNS prompts, GlobalShortcuts on Wayland.

### 5.4 Minimum for a first Windows build

Everything in 5.3 that is not Linux-specific, plus:

1. **Compile**: seams 1 and 4 of 5.2 (`PrivateFs` / `LocalIpc`, `ProcessControl`) so
   `cargo build --workspace` passes with the 0.2 modules in it.
2. **Never lie about stopping** (5.1 step 4, then the Job Object).
3. **Find the tools**: `LaunchEnv` and `ToolLocator` (4.2, 4.4, 4.6), `ExeId` (2.1-2.6) so
   `claude.exe` / `codex.exe` / `.cmd` shims are recognised and `git` is found.
4. **Agent bridge** (5.5): a named-pipe `LocalIpc`, or the bridge, hooks and the Claude
   statusline capture are off and the UI says so (4.7 is **U** and needs Git Bash).
5. **Data dir decision**: `%LOCALAPPDATA%` for the database (1.1).
6. **Updater** (14.2, 14.3): re-enable the `zip` feature once a fixed version ships, an NSIS
   or MSI target, `windows-x86_64` in the feed, Authenticode signing. Until then the first
   Windows release is download-and-reinstall, and must say so.
7. **Ports** (11.1): confirm `netstat2` on Windows or show "ports unknown" (already honest).
8. **Tests**: a Windows `cargo test` job for the crates that claim Windows support. Most
   existing tests are `cfg(unix)` or use `/bin/sh` (section 1), so the first Windows test
   run will be small on purpose and grow with the seams.
9. State that **WSL sessions are not tracked** in the first release (section 4).

Deferred without harm: Claude and Codex desktop detection on Windows (data dirs **U**),
Go to it (the Windows foreground rules make it unreliable), memory pressure, thermal,
power, window mirroring, native crash reports.

### 5.5 Open questions that need a lab (read-only, sacrificial profiles)

| # | Question | Needs |
|---|---|---|
| Q1 | Does `release/0.2` compile on Windows? Which crates break? | A CI run, no hardware |
| Q2 | Does `netstat2` list listeners on Windows? On Linux with IPv6? | A Windows VM |
| Q3 | How does `portable-pty` treat `PATH` vs `Path`, and do `.cmd` shims spawn through ConPTY? | A Windows VM |
| Q4 | Claude desktop: Windows and Linux data dir, process names, `claude-code-sessions` layout | A Windows and a Debian machine with the app installed |
| Q5 | Codex desktop on Windows and Linux: install location, exe name, data dir, URL scheme | Same |
| Q6 | Does Claude Code run a statusline command through Git Bash or PowerShell on Windows? | A Windows VM |
| Q7 | Does `tauri-plugin-notification`'s `on_action` fire on Windows and Linux? | Either |
| Q8 | Does `trusted_executable` reject the bridge binary inside an AppImage mount? | A Linux box |
| Q9 | How does Claude Code mangle a Windows cwd into the `projects/` folder name? | A Windows VM |
| Q10 | OpenCode user-config and state paths on Windows; Kimi and Grok credential protection on Windows | A Windows VM |

---

## Appendix A. Evidence log for web claims

Accessed 2026-10-27 by `WebFetch` (page text summarised by the tool; quotes are the tool's
and are short). The first four links are Anthropic's own docs.

| Claim | Source | Label |
|---|---|---|
| Claude Code runs on macOS 13+, Windows 10 1809+, Ubuntu 20.04+, Debian 10+, Alpine 3.19+; native Windows installer; WinGet; WSL 1/2; native binary `%USERPROFILE%\.local\bin\claude.exe` | https://code.claude.com/docs/en/setup.md | VP |
| Credentials: macOS Keychain (file fallback), Linux `~/.claude/.credentials.json` 0600, Windows `%USERPROFILE%\.claude\.credentials.json`; `CLAUDE_CONFIG_DIR` relocates it and keys the Keychain entry | https://code.claude.com/docs/en/authentication.md | VP |
| `~/.claude` resolves to `%USERPROFILE%\.claude` on Windows; `CLAUDE_CONFIG_DIR` moves every path | https://code.claude.com/docs/en/claude-directory.md | VP |
| Claude Desktop on Linux is beta, Debian-based only (Ubuntu 22.04+, Debian 12+), apt, no self-update, login in the desktop keyring, Computer Use not available | https://code.claude.com/docs/en/desktop-linux.md | VP |
| Windows Code tab can run sessions inside a WSL 2 distribution | https://code.claude.com/docs/en/desktop-wsl.md | VP |
| Windows desktop app data at `%APPDATA%\Claude`; Linux path inconsistent across sources | WebSearch "Claude desktop app Windows data folder %APPDATA%\Claude …" (inventivehq.com, glama.ai, claude.com/docs/third-party). Only the MCP config file location is evidenced, not the Code-tab session store | VS, Linux **U** |
| Codex on Windows: native desktop app, CLI, IDE extension; sandbox modes; `CODEX_HOME` example `C:\Users\alice\.codex` | https://learn.chatgpt.com/docs/windows/windows-sandbox (redirect from developers.openai.com/codex/windows) | VP |
| Codex credentials in `~/.codex/auth.json` or the OS credential store; `cli_auth_credentials_store` values | https://learn.chatgpt.com/docs/auth | VP |
| OpenCode data and logs paths on macOS/Linux/Windows; Windows runs natively, WSL recommended; managed config dirs | https://opencode.ai/docs/troubleshooting/, /docs/windows-wsl/, /docs/config/ | VP |
| Kimi Code data under `~/.kimi-code` on macOS/Linux/Windows; `KIMI_CODE_HOME`; `.agents` stays under the OS home | https://moonshotai.github.io/kimi-code/en/configuration/data-locations.md | VP |
| Grok Build prebuilt binaries for macOS, Linux, Windows; Windows source builds best-effort; `GROK_HOME` resolution and `home_dir()` choice | https://raw.githubusercontent.com/xai-org/grok-build/main/README.md and `crates/codegen/xai-dirs/src/lib.rs` | VP |
| `dirs::data_dir`: Linux `$XDG_DATA_HOME` or `~/.local/share`; macOS `~/Library/Application Support`; Windows `{FOLDERID_RoamingAppData}` | https://docs.rs/dirs/latest/dirs/fn.data_dir.html | VP |
| `sysinfo` supports Linux, macOS, Windows (and others); on unsupported systems returns empty values | https://docs.rs/sysinfo/latest/sysinfo/ | VP |
| `keyring` crate lists optional back ends for Apple, Windows, Linux Secret Service and keyutils; feature names and limits not shown | https://docs.rs/keyring/latest/keyring/ | VP (weak) |
| Tauri bundle formats: Windows installer / Store; Linux deb, rpm, AppImage, Snap, Flatpak, AUR; macOS app, dmg | https://v2.tauri.app/distribute/ | VP |
| Tauri deep links: install-time schemes; runtime `register` on Linux and Windows only; links arrive as argv and need the single-instance `deep-link` feature | https://v2.tauri.app/plugin/deep-linking/ | VP |
| Tauri notifications on Windows work only for installed apps | https://v2.tauri.app/plugin/notification/ | VP |
| Tauri autostart: Windows, Linux, macOS listed; only macOS mechanism named | https://v2.tauri.app/plugin/autostart/ | VP (weak) |
| portable-pty: the docs.rs page confirms Unix PTYs and a `PtySystem` trait chosen at runtime "important on Windows systems"; ConPTY by name not on the page | https://docs.rs/portable-pty/latest/portable_pty/ | VP (weak); ConPTY = **I** |

**Negative evidence (NE).** Search log, 2026-10-27:
- *Codex Linux desktop app*: fetched `https://learn.chatgpt.com/docs/linux-desktop-app` (404, guessed URL); the Windows page's navigation lists a "Linux desktop app" page whose content was not read. Not found by this search; not a statement that it does not exist.
- *Claude desktop Linux data dir, official*: one WebSearch; no Anthropic page named the Code-tab store. Not found by this search.
- *Grok Build `GROK_HOME` on Windows in the README*: not stated (the README only points at its user guide); the source file above answers it.
- *OpenCode user-level config path on Windows*: not stated on `docs/config`; not found on `docs/troubleshooting`.

Carried from the repo's research (2026-10-21, not re-read): Codex `cli_auth_credentials_store`
default `file`; OpenCode desktop download targets; Kimi desktop OS list and `kimi-code://`;
Grok auth file mode and no Keychain use; Claude Code's `claude agents --json` and deep link.

## Appendix B. Method and limits

- Two read-only subagents each swept a set of crates and the build tree, listing every
  `#[cfg(...)]`, every `std::os::unix`, every absolute macOS tool or `.app` path and every
  `/`-separator assumption in non-test code. I re-read the load-bearing claims myself:
  `file_trust::me()` is `cfg(unix)` only (`file_trust.rs:9-13`); `runtime/launch.rs:80-99`
  and `agent_bridge/{mod,server}.rs` have no `cfg`; `user_path.rs:44-54` has no
  `USERPROFILE`; `GIT_PLACES` is Unix-only; the updater feed has only `darwin-*`; the
  `zip` feature is off with an explicit comment; `is_cli_process` compares to `claude`
  exactly; CI jobs and their results on `main`.
- **Not done:** no build on Windows or Linux, no run of any test, no read of the real
  user data, no sign-in, no Keychain access. Whether `release/0.2` compiles on Windows is
  the first open question (Q1).
- `origin/release/0.2` moved by one documentation commit (`d5d0ac92`) while this was written;
  it does not touch the files cited.
