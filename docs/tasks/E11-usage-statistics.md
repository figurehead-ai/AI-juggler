# E11 — Usage statistics (opt-in)

> Source decision: [D-19](../review/decision-log.md) (2026-10-18) and
> [ADR-0015](../adr/0015-usage-statistics.md). The server half is the
> Figurehead usage-statistics contract; the closed schema both sides
> enforce is copied unchanged at [`stats/schema.v1.json`](../../stats/schema.v1.json).
>
> **Name:** "usage statistics" ("usage stats" in UI copy), code prefix
> `stats`. Never "telemetry": in AI Juggler that word means the status
> sync between paired Macs (E7).
>
> **Law for every task here:**
> - **Off by default.** Until the user chooses, nothing is counted, no
>   `stats/` folder exists, and no request is made.
> - **Aggregated on the Mac.** Only weekly counters, never raw events.
> - **No identifiers of any kind.**
> - **Never collected:** prompts, transcripts, code, paths, project or
>   Account names, machine or user names, setting values outside the
>   schema's enums, error text. Nothing is ever read from, derived from
>   or hashed from vendor credential or identity files.

### AJ-E1101 Decision D-19, ADR-0015, doc amendments, the schema copy | Status: DONE-EVIDENCED | Priority P0 | Effort S | Depends: none
**Goal**: Record the usage-statistics decision and make every existing document say the same thing before any code exists.
**User story**: As a maintainer, I can point to one decision, one ADR and one schema that define exactly what may ever leave the Mac.
**Requirements**:
1. `docs/review/decision-log.md` SHALL gain D-19 (opt-in usage statistics with three modes, on-device aggregation, weekly reports with no identifier, sampling, a closed schema, a separate crash-report setting, overrides, consent versioning, personal sync, abuse protection without identity), and `docs/adr/0015-usage-statistics.md` SHALL record the architecture.
2. SPEC §1 (Privacy row), SPEC §8, SECURITY.md and the README privacy line SHALL describe usage statistics as opt-in, aggregated on the device, identifier-free and documented in `docs/usage-stats.md`.
3. `stats/schema.v1.json` SHALL be a byte-identical copy of the server's schema. `scripts/stats_schema_diff.sh` SHALL compare the two copies locally, and the release checklist (`docs/RELEASING.md`) SHALL run it.
**UI/UX reasoning**: n/a (documents); copy uses the user's words ("usage stats"), never "telemetry" (NN-g #2, match the real world).
**Performance budget**: n/a.
**Quality gates**: the terminology lint stays green; the evidence and link checks pass.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1101/manifest.md` listing each amended line and the schema checksum; pass = the diff script reports identical.
**Visual testing**: none.
**Files to touch**: `docs/review/decision-log.md`, `docs/adr/0015-usage-statistics.md`, `docs/adr/README.md`, `docs/SPEC.md`, `SECURITY.md`, `README.md`, `stats/schema.v1.json`, `scripts/stats_schema_diff.sh`, `docs/RELEASING.md`.
**Research basis**: the usage-statistics specification (Figurehead, approved 2026-10-16, amended 2026-10-17).
**Handover note**: The schema is the one file copied from the website repo; everything else is written here in our own words. Never put server internals (limits, firewall rules, costs) in this repo.

### AJ-E1102 The `juggler-stats` crate: modes, overrides, counting, local store, the cargo feature | Status: DONE-EVIDENCED | Priority P0 | Effort M | Depends: AJ-E1101
**Goal**: A small crate that counts closed, typed events on the Mac — and does nothing at all until the user opts in.
**User story**: As a user who never opted in, I have no stats folder and no stats code runs; as one who did, counting never slows the app.
**Requirements**:
1. Modes `off` (default) / `local` / `share`, stored as `{mode, decided_at, consent_version}` in the app settings. The effective mode applies the overrides in order: built without the `stats` cargo feature → not included; `DO_NOT_TRACK` set to anything but empty or `0` → off; `AI_JUGGLER_STATS=off` → off, `=local` caps at local; `CI` set or a debug build → never upload; otherwise the user's mode. Each override carries its reason.
2. `StatEvent` / `StatGauge` SHALL be closed Rust enums with no `String` fields, one variant per schema metric, with a parity test proving they equal `stats/schema.v1.json` (names, dimensions, enum values, buckets).
3. `record()` SHALL never block or fail: a bounded channel (1,024) drops when full and counts `stats.dropped`; a background flush writes `stats/counters.sqlite` every 30 s and on quit. In `off`, `record()` returns immediately and no `stats/` folder is created.
4. Switching to `off` SHALL delete the whole `stats/` folder (counters, reports, salt, upload log) and say so ("Usage stats are off. Local stats deleted.").
5. Building without the `stats` feature SHALL leave the crate out of the binary (`cargo tree` check).
**UI/UX reasoning**: Privacy by default (Apple HIG, Privacy): nothing happens until an explicit choice.
**Performance budget**: `record()` < 1 µs median, never waits on I/O; the flush < 5 ms for a week of counters.
**Quality gates**: parity test; off-mode no-folder test; channel-full drop test.
**Evidence/validation proof**: `cargo test -p juggler-stats` log in `docs/qa/evidence/AJ-E1102/`; pass = all green, plus the record() benchmark line.
**Visual testing**: none (AJ-E1105 owns the UI).
**Files to touch**: `crates/juggler-stats/`, `stats/schema.v1.json`, `scripts/gen_stats_events.py`, workspace `Cargo.toml`, `crates/juggler-app/Cargo.toml` (feature).
**Research basis**: spec §3–§4.
**Handover note**: The enums are generated from the schema by a committed script; regenerate rather than hand-edit, and keep the parity test as the proof.

### AJ-E1103 Wire the events into the app | Status: DONE-EVIDENCED | Priority P0 | Effort M | Depends: AJ-E1102
**Goal**: Count what people actually use: Instances, switching, usage clocks, cleanup, onboarding, Fleet, the interface, reliability and performance, exactly as the schema lists them.
**User story**: As the maintainer, I can see which features matter, without ever seeing anyone's work.
**Requirements**:
1. Every schema metric SHALL have exactly one emitting site or a documented "not emitted yet" reason; Provider ids map to the schema enum (`other` otherwise), models to `model_family`.
2. Gauges SHALL be snapshotted when a week closes, from the read model, through the bucket helpers (counts → `0…10+`; versions → `major.minor` or `other`).
3. Token and minute totals SHALL be counted only where Measured (D-06) and rounded at assembly.
4. No emitting site SHALL pass anything derived from a path, label, name, setting value outside an enum, or error text.
**UI/UX reasoning**: n/a.
**Performance budget**: zero measurable cost in `off`; ≤ 1 µs per event otherwise.
**Quality gates**: a coverage test lists every metric with its emitting site or reason.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1103/manifest.md` with the metric → site table; pass = every metric accounted for.
**Visual testing**: none.
**Files to touch**: `crates/juggler-app/src/` (commands, link, fleet, backup, updates, onboarding paths), `ui/src/lib/bridge.ts` (UI events).
**Research basis**: spec §7.
**Handover note**: An event that would need free text is a schema change — ask the maintainer; never widen a type.

### AJ-E1104 Weekly assembly, sampling, config, upload and backoff | Status: DONE-EVIDENCED | Priority P0 | Effort M | Depends: AJ-E1102
**Goal**: Turn a closed ISO week (UTC) into one validated report and upload it later, at a random time, only when the user shares and the service accepts.
**User story**: As a sharing user, one small report a week leaves my Mac, and I can read it first.
**Requirements**:
1. Each closed week SHALL be assembled once: gauges snapshotted, rounding applied, counters and histograms clamped to the schema caps (clamps counted in `stats.truncated`), validated, then written to `pending/` with an upload time 0–144 h after the week closed. A partially consented week, a week assembled in `local`, an invalid report or one older than 13 days goes to `skipped/` with its reason.
2. Sampling SHALL use `sha256(salt ‖ week)` against the config's `sample_rate`, with the salt never sent and the applied rate recorded in the report.
3. The config SHALL be fetched at most once a day when a report is due, reused for up to 7 days, and obeyed (`enabled`, version range, schemas, `pow_bits`). The upload URL SHALL be compiled in, never read from the config.
4. Upload: one report per hourly tick, oldest first, gzip JSON, `Content-Type: application/json`, `User-Agent: AIJuggler/<version>`, no cookies, 10 s timeout. `202`/`200` → `sent/`. `400`/`404`/`413` → `skipped/`. `429`/`503` → retry after `Retry-After` (cap 72 h). Network or other `5xx` → backoff 1 h, 3 h, 9 h, 24 h, 48 h (±20 %). Give up after 6 attempts or at expiry.
5. Release builds SHALL ignore `AI_JUGGLER_STATS_ENDPOINT`; debug builds may upload only there.
**UI/UX reasoning**: Visibility (NN-g #1): every report is readable before and after it is sent.
**Performance budget**: assembly < 50 ms; a report < 4 KB typical, ≤ 8 KB hard (truncation order `ui.*`, `perf.*`, then the rest).
**Quality gates**: fake-clock tests for the schedule, `Retry-After`, the attempt cap and expiry; sampling ±2 % over 10,000 salts at 0.1 and 0.5; a worst-case week ≤ 8 KB.
**Evidence/validation proof**: test logs in `docs/qa/evidence/AJ-E1104/`; plus a run against the local stats endpoint (debug build).
**Visual testing**: none (AJ-E1105).
**Files to touch**: `crates/juggler-stats/src/` (week, report, upload), `crates/juggler-app/src/` (tick).
**Research basis**: spec §5–§6.
**Handover note**: Production collection is switched off until launch: the config says `"enabled": false`, so a sharing user's reports are filed under `skipped/` as `server_disabled`, and a direct upload gets `503` with `Retry-After` (handled too). Test against the local stats server as the website's usage-stats prompt describes, never production.

### AJ-E1105 Consent in onboarding and Settings ▸ Privacy | Status: DONE-EVIDENCED | Priority P0 | Effort M | Depends: AJ-E1102
**Goal**: Ask once, fairly, and let the user see and control everything afterwards.
**User story**: As a new user, I choose share, local-only or off with no option nudged; later I can read exactly what was counted and sent, and delete it.
**Requirements**:
1. Onboarding's first-run policy step SHALL show the three options with equal weight and nothing preselected; leaving without choosing keeps `off`. A consent-version bump asks again with the current mode preselected, sending nothing until answered.
2. Settings ▸ Privacy SHALL show the mode (with the override reason when one applies), this week's counters grouped by category, the pending/sent/skipped reports with week, status and reason (each opening its exact JSON read-only, with Copy), the next upload time, "Delete local stats", "Open metrics documentation", and the crash-report setting.
3. States: off, local, share, overridden, not included in this build, and an upload error in plain words with the retry time.
**UI/UX reasoning**: Equal-weight choices and no preselection (NN-g #3/#5; dark-pattern avoidance); the read-only JSON viewer is recognition over recall (NN-g #6).
**Performance budget**: the Privacy screen renders < 200 ms with 12 weeks of reports.
**Quality gates**: vitest for each state; axe 0 in both themes.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1105/manifest.md`; gallery captures of every §8 state.
**Visual testing**: the consent step and Settings ▸ Privacy in every state, 1280×800 and 1024×680, both themes.
**Files to touch**: `ui/src/screens/OnboardingWizard.tsx`, `ui/src/screens/Settings.tsx`, a Privacy card component, the bridge, `crates/juggler-app/src/stats.rs`.
**Research basis**: spec §3, §8.
**Handover note**: The three option labels and their one-line explanations are part of the consent; change them only with a consent-version bump.

### AJ-E1106 Guardrail tests and the public metrics document | Status: TODO | Priority P0 | Effort M | Depends: AJ-E1103, AJ-E1104, AJ-E1105
**Goal**: Prove every promise in tests, and publish exactly what is collected.
**User story**: As a sceptical user or reviewer, I can read the full list of metrics and see a test for each promise.
**Requirements**:
1. Tests SHALL cover: default silence (no folder, zero requests); the three overrides with their reasons; no forbidden content in an assembled report from a seeded week (an Account label, a project under `$HOME`, a private model name, an error containing a secret-shaped string, the home path and user name); no reads of vendor credential paths during stats code; schema parity and validation; consent windows; sampling; backoff and expiry; off-means-gone; the feature flag (`cargo tree`); the 8 KB worst case; synced-never-doubles; the puzzle vectors; the user agent on every upload.
2. `docs/usage-stats.md` SHALL be generated from the schema (every metric with its dimensions and values), with the modes, the never-collected list, every way to turn it off, server retention and a sample report; CI SHALL fail if it drifts from the schema.
**UI/UX reasoning**: n/a.
**Performance budget**: the guardrail suite ≤ 30 s.
**Quality gates**: all of the above green in `just verify`.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1106/manifest.md` mapping each guardrail to its test.
**Visual testing**: none.
**Files to touch**: `crates/juggler-stats/tests/`, `crates/juggler-app/tests/`, `scripts/gen_usage_stats_doc.py`, `docs/usage-stats.md`, `justfile`.
**Research basis**: spec §11–§12.
**Handover note**: A guardrail that needs the network is wrong; use a recording transport.

### AJ-E1107 Crash reports behind their own setting | Status: TODO | Priority P2 | Effort M | Depends: AJ-E1105
**Goal**: Optional crash reports, separate from usage statistics, scrubbed on the Mac.
**User story**: As a user, I decide whether crash reports are never sent, sent after I review them, or always sent.
**Requirements**:
1. Setting `never` (default) / `ask` / `always`; `DO_NOT_TRACK` forces `never`.
2. `ask` keeps the report locally and asks on next launch with a full read-only preview: Send / Don't send / Always send.
3. Scrubbing on the Mac before anything is sent: no PII defaults, no server name, user or environment, the home directory replaced with `~`, no query strings or URLs, app-internal breadcrumbs only (≤ 50), no attachments other than the minidump.
4. On-device limits: one report per crash signature per day, five per day in total.
5. The reporting service's address is compiled into release builds only; minidumps are tested in a signed, notarized build before they are enabled.
**UI/UX reasoning**: Preview before send (NN-g #1, #3).
**Performance budget**: no cost when `never`.
**Quality gates**: a scrubbing test over a seeded crash (home path, user name, a secret-shaped string); limits test.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1107/manifest.md`; pass = the scrub test green and a notarized-build minidump test recorded.
**Visual testing**: the ask dialog and the setting, 1280×800 both themes.
**Files to touch**: `crates/juggler-app/src/crash.rs`, Settings ▸ Privacy.
**Research basis**: spec §10.
**Handover note**: Needs a signed, notarized build to verify minidumps — that is the maintainer's step (AJ-E1010).

### AJ-E1108 Puzzle solver and the user agent | Status: DONE-EVIDENCED | Priority P1 | Effort S | Depends: AJ-E1104
**Goal**: Keep honest uploads working when the service asks for a proof of work, at no cost the rest of the time.
**User story**: As a sharing user, my reports keep arriving even when the service is protecting itself.
**Requirements**:
1. When the config's `pow_bits` is above 0, find a nonce (lowercase hex, ≤ 32 chars) such that `sha256("ajstats-pow-v1:" ‖ report_id ‖ ":" ‖ week ‖ ":" ‖ nonce)` starts with that many zero bits, on a low-priority thread after the upload delay; give up after 60 s of CPU and retry next tick; above 28 bits, postpone 24 h.
2. `403 {"error":"pow","bits":N}` SHALL refetch the config and solve again on the next tick (it counts as an attempt). With `pow_bits` 0 the report has no `pow` field.
3. Every stats upload SHALL send `User-Agent: AIJuggler/<version>`.
**UI/UX reasoning**: n/a.
**Performance budget**: 20 bits ≈ 0.1 s on one Apple-silicon core.
**Quality gates**: vectors that the server's check accepts; the 403 path; the user-agent test.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1108/manifest.md`.
**Visual testing**: none.
**Files to touch**: `crates/juggler-stats/src/pow.rs`, `crates/juggler-stats/src/upload.rs`.
**Research basis**: spec §5.5.
**Handover note**: The nonce is bound to the report id and week; never reuse one.

### AJ-E1109 Personal stats sync | Status: TODO | Priority P1 | Effort M | Depends: AJ-E1104, AJ-E1202
**Goal**: Let a signed-in user see their own weekly stats in their Figurehead dashboard — instead of, never as well as, the anonymous report.
**User story**: As a user with a linked Mac, I can choose to see my stats on figurehead.ai; only I can see them, and they go when my account goes.
**Requirements**:
1. Offered only when the mode is `share` and this Mac is linked: "Show my stats in my Figurehead dashboard", unticked by default, with "Synced to <account email>" when on.
2. A week is synced only if the box was ticked for the whole week; it then goes to the account endpoint with the device token, unsampled, with no puzzle, and **never** also to the anonymous endpoint.
3. `401` SHALL untick the box, skip the report (reason `unlinked`), say "This Mac is no longer linked, so stats sync is off.", and never fall back to anonymous without the user choosing.
**UI/UX reasoning**: Explicit, separate consent for linking data to an identity (NN-g #3).
**Performance budget**: as AJ-E1104.
**Quality gates**: synced-never-doubles; the 401 path; the whole-week rule.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1109/manifest.md`; plus a run against the website's local dev server.
**Visual testing**: the checkbox states inside Settings ▸ Privacy (AJ-E1105's captures).
**Files to touch**: `crates/juggler-stats/src/upload.rs`, `crates/juggler-app/src/stats.rs`, the Privacy card.
**Research basis**: spec §3.6.
**Handover note**: Two copies of the same week would let the anonymous one be matched to an account — the never-both rule is the privacy guarantee, not an optimisation.
