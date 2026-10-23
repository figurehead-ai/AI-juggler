# Evidence manifest — release-0.1-real

0.1 acceptance §3, the real-data pass (`docs/qa/0.1-acceptance.md`), on the maintainer's Mac.
Read-only, 2026-10-23, claude-opus (QA session).

**Heads:**
- 3.1–3.5 ran on `release/0.1` `f4181eee`.
- The E1538 estimate was read through the bridge of `qa/0.1-acceptance` `f80fa4f3`, because
  `release/0.1`'s bridge refuses `outside_search_estimate` and `outside_search_status`.
- The measured index was built in a throwaway copy, with the same estimate code (`release/0.1`).

**How:**
- **Dev build:** `--features dev-bridge`, with its own identifier (`ai.figurehead.juggler.dev.qa031`,
  dev URL port 5197), next to the coordinator's replica soak, whose processes weren't touched.
  Started Finder-like with `env -i`.
- **Accounts:** `qa_real_readonly` (this PR) brought the 7 real setups in where they are, with
  redacted labels (Account A–G), into a fresh `AI Juggler Dev` folder. It then read projects,
  conversations, usage and quota, and sampled This Mac 7 times over a minute. At each sample it
  recorded `ps` (pid, ppid, rss, cpu, elapsed, file name only) and `vm_stat` with the pressure
  level, swap and `hw.memsize`.
- **Oracle:** `scripts/qa/real_truth.py --compare` (this PR) checked that output against the
  tools' own files. Its output is `compare-f4181eee.txt`: 80 OK, 7 MISMATCH, 3 INFO. The bridge
  reads are in `bridge-checks-f4181eee.txt`.
- **Outputs:** redacted counts only; no secret preview, path or name.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| 3.1 projects, conversations, live sessions | `qa_real_readonly`, then `real_truth.py --compare`; `conversations_list` through the bridge | every line OK | Exact for 6 accounts' projects, conversations and live sessions, and the live set (0 unaccounted). 26 project rows = 26, and 23 exact. **Account G (Codex): 2,361 conversations against 1,962**, and 3 projects off by the same 399. Release/0.1 lists Codex subagent threads (`forked_from_id`, `source.subagent`) as conversations; Claude Code's subagents aren't listed. Through the bridge the counts equal the harness's. | FAIL | f4181eee | 2026-10-23 | claude-opus |
| 3.2 Runway's and Usage's inputs | the same compare: 5 h / 7 d window tokens, Codex all time, cost rows per session | within 5% | All 14 window lines inside the live-writer band (×0.98–1.00). Codex all time ×1.00 (19.64 B over 2,241 sessions). **Cost rows: 12 of 2,300 sessions over 5% (max ×1.65)**, all Claude conversations a switch carried into a second account, read high: P19, still open. 761 of 2,613 rows priced; every unpriced one is `codex-auto-review`. | FAIL | f4181eee | 2026-10-23 | claude-opus |
| 3.3 Codex `rate_limits` as recorded | the same compare: the weekly and 5-hour lines | equal to the newest reading | Weekly 39.0% used = the rollouts' 39.0%, resets Thu 13:09. No 5-hour window, as Codex reports (`secondary` null). | PASS | f4181eee | 2026-10-23 | claude-opus |
| 3.4 This Mac against `ps` and `vm_stat` | the same compare; `this_mac` through the bridge, with `vm_stat` right after | `ps`: each tree within 15%; `vm_stat`: memory used within 3% of physical memory; total and pressure equal | `ps`: 8 of 8 trees, tools 20.7 GB = 20.7 GB, groups 8 = 8 (OK). Total 103.1 GB = `hw.memsize`; pressure "warning" = level 2 (OK). **Memory used reads 3.0–3.4 GB low (3.2% of 103 GB) at every sample, and 2.8% through the bridge.** The compressed term is `vm.compressor_bytes_used` (the compressed data, 35.2 GB). `vm_stat`, `top` and Activity Monitor count the pages the compressor occupies, `vm_statistics64.compressor_page_count` (38.6 GB). | FAIL | f4181eee | 2026-10-23 | claude-opus |
| 3.5 the real data unchanged | `snap.sh` before and after (paths, sizes, mtimes of `~/.claude`, `~/.codex`, ClaudeCodeProfiles, and every `settings.json`, `config.toml` and `.claude.json`) | nothing added or removed; no config change of ours | 199 entries before and after: none added or removed (`snapshot-diff-f4181eee.txt`). Six config files have a new mtime at the same size, all written by the vendors on their own clocks: `remote-settings.json` ×4 (Claude Code's sync), Codex's `config.toml` (the Codex app rewrites it all day), and one profile's `.claude.json` (its periodic write). 36 other entries are live sessions' own files. The product never writes these files. | PASS | f4181eee | 2026-10-23 | claude-opus |
| E1538 the size estimate | `outside_search_estimate` through the bridge (`f80fa4f3`); `qa_outside_estimate` (this PR) builds the whole index in a throwaway copy of the Dev data folder, measures it, turns it off, and the copy is deleted | within ×0.5–×2 of the built index | **The estimate is about 3.5× too high.** Through the bridge: 2.90 GB for 2,443 sessions (1.1 s). In the copy, with the same code: 3.14 GB for 2,441 sessions. The built index is **0.89 GB**: 2,434 of 2,441 sessions indexed in 8 passes, 320 s, with 5.3 GB peak memory for the whole test. Seven sessions stayed unindexed when the passes ended. That makes the built figure slightly low, but nowhere near ×3. Off by ×3.3–×3.5. Turning it off deleted the index, and the copy was deleted. (`outside-search-estimate-*.json`) | FAIL | f80fa4f3 | 2026-10-23 | claude-opus |

## Not covered

- The Codex conversation count depends on a product decision: whether a subagent thread is a
  conversation of its own. If it is, the oracle should count it, and Claude Code's subagents should
  be listed too.
- Usage's priced totals: `codex-auto-review` has no price row (known).
- The `f80fa4f3` Dev app exited on its own once, about 2 minutes in, with no log line, crash report or
  jetsam report, while memory pressure was at "warning" (the soak was running). A relaunch stayed up, and
  the estimate above is from it.
- Seven sessions stayed unindexed after the build: not looked into here.

## Redaction checklist

- [x] No credentials, tokens or keys in any file here (outputs are counts and the app's own words)
- [x] Accounts as Account A–G, projects as Project N; no path
- [x] No transcript text
