# Evidence manifest — AJ-E2309

Task: `docs/tasks/E23-session-runtime.md` AJ-E2309, Docs, Privacy and terms
lines, evidence. Built on `e23-runtime` (2026-10-25). Checked again,
requirement by requirement, on 2026-11-06 (release/0.4.1). Status:
**DONE-EVIDENCED** (agent self-check, 2026-11-06).

| Req | Result | Evidence |
|---|---|---|
| R1 | PASS | `docs/research/codex.md` line 127, "## §9 addendum (2) — app-server's rate limits are documented (D-40, 2026-10-25)"; line 131: "`account/rateLimits/read` and the notification `account/rateLimits/updated` are stable app-server methods", with the windows' fields and the recorded surface `crates/juggler-adapters/tests/fixtures/codex-app-server/surface-0.163.0.json`. It says it corrects the §9 bullet that called the feed undocumented. |
| R2 | PASS | `docs/adr/0003-automation-policy-compliance-guardrails.md`, "Amendment, 2026-10-25" (line 31): quotes Anthropic's *Legal and compliance* page word for word (lines 35–39, linking research `session-runtime.md` §2.1), and point 8 (line 45) is "Answers are always the user's (ADR-0022 decision 5)". |
| R3 | PASS | **Privacy lines naming the runtime:** `ui/src/lib/trust-claims.json` `runtime-held`, `runtime-launch`, `no-answer-by-itself`, `no-bypass`, each with its tests. `juggler-app/tests/trust.rs` `every_trust_claim_names_tests_that_exist` fails unless every named test exists (and isn't ignored) and `docs/guide/trust.md` says each line word for word. **Terms lines:** each driven runtime's `terms_note` (`claude_stream/mod.rs`, `codex_appserver/mod.rs`); `every_tool_names_its_terms_and_the_preview_matches` checks each note is in the preview and in `docs/qa/evidence/AJ-E1518/terms-claims-register.md`. The page: `ui/src/screens/privacy-page.test.tsx` ("keeps each claim's tests behind it", "says how each tool's sessions run under its terms, only for a tool that has a runtime"). |
| R4 | PASS | `docs/guide/sessions-and-the-terminal.md`: **runtimes**, "## How a session runs" (line 84: in a terminal, or through the tool's own app-server); **answering**, "## Answering a session's questions" (line 132: "Nothing is answered until you click"); **the Beta and its gates**, "### Claude Code without a terminal (Beta)" (line 149), which lists the four gates ("It stays a Beta until all four of these hold"). |
| R5 | PASS | The manifests exist: `docs/qa/evidence/AJ-E2302/` through `AJ-E2308/manifest.md` and `AJ-E2310/manifest.md`. (Those tasks are still IN-PROGRESS in `docs/tasks/E23-session-runtime.md`, and their manifests are in the older `Req.N` format, so `release_scope.py` doesn't count them as proven. That is their own work, not this requirement's.) |

Run on 2026-11-06: `cargo test -p juggler-app --test trust` (14 passed, `every_trust_claim_names_tests_that_exist` and `every_tool_names_its_terms_and_the_preview_matches` among them); `pnpm exec vitest run src/screens/privacy-page.test.tsx` (passed).

## The first part (2026-10-25, e23-runtime)

Task: `docs/tasks/E23-session-runtime.md` AJ-E2309, Docs, Privacy and terms lines, evidence. Branch `e23-runtime`. Status: **DONE-EVIDENCED**: the codex.md addendum, ADR-0003's amendment, four Privacy lines and two terms lines tied to tests, the guide sections, and these manifests.

### What exists now

`docs/research/codex.md` (§9 addendum (2)); `docs/adr/0003-*.md` (points 7–8); `ui/src/lib/trust-claims.json` and `docs/guide/trust.md` (`runtime-held`, `runtime-launch`, `no-answer-by-itself`, `no-bypass`); each driven runtime's `terms_note` on the Privacy page; `docs/qa/evidence/AJ-E1518/terms-claims-register.md`; `docs/guide/sessions-and-the-terminal.md`.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| Req.1 codex.md: app-server's rate limits are documented | `docs/research/codex.md` §9 addendum (2) | sources and lab cited | written | PASS | — | 2026-10-25 | agent |
| Req.2 ADR-0003 cites Anthropic's wording and ADR-0022 decision 5 | `docs/adr/0003-automation-policy-compliance-guardrails.md` | quotes word for word | written | PASS | — | 2026-10-25 | agent |
| Req.3 Privacy lines, each with tests that exist | `cargo test -p juggler-app --test trust` (`every_trust_claim_names_tests_that_exist`, `every_tool_names_its_terms_and_the_preview_matches`); `cd ui && npx vitest run src/screens/privacy-page.test.tsx` | every test named runs; preview and register match | pass | PASS | — | 2026-10-25 | agent |
| Req.3 the held conversation is bounded and never written | `cargo test -p juggler-app --lib the_conversation_is_held_in_memory_and_bounded` | only the latest kept; no writer in the driver | pass | PASS | — | 2026-10-25 | agent |
| Req.4 guide: runtimes, answering, the Beta and its gates | `docs/guide/sessions-and-the-terminal.md`; plain-words and terminology lints | clean | pass | PASS | — | 2026-10-25 | agent |
| Req.5 manifests for AJ-E2301..E2309 | `scripts/evidence_check.sh` | OK | pass | PASS | — | 2026-10-25 | agent |

### Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here (secret scan ran)
- [x] Paths hashed or generalised where they reveal private layout
- [x] Transcript excerpts scrubbed of personal content (none: fixtures only)
