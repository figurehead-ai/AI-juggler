# Evidence manifest — AJ-E2309

Task: `docs/tasks/E23-session-runtime.md` AJ-E2309, Docs, Privacy and terms lines, evidence. Branch `e23-runtime`. Status: **DONE-EVIDENCED**: the codex.md addendum, ADR-0003's amendment, four Privacy lines and two terms lines tied to tests, the guide sections, and these manifests.

## What exists now

`docs/research/codex.md` (§9 addendum (2)); `docs/adr/0003-*.md` (points 7–8); `ui/src/lib/trust-claims.json` and `docs/guide/trust.md` (`runtime-held`, `runtime-launch`, `no-answer-by-itself`, `no-bypass`); each driven runtime's `terms_note` on the Privacy page; `docs/qa/evidence/AJ-E1518/terms-claims-register.md`; `docs/guide/sessions-and-the-terminal.md`.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| Req.1 codex.md: app-server's rate limits are documented | `docs/research/codex.md` §9 addendum (2) | sources and lab cited | written | PASS | — | 2026-10-25 | agent |
| Req.2 ADR-0003 cites Anthropic's wording and ADR-0022 decision 5 | `docs/adr/0003-automation-policy-compliance-guardrails.md` | quotes word for word | written | PASS | — | 2026-10-25 | agent |
| Req.3 Privacy lines, each with tests that exist | `cargo test -p juggler-app --test trust` (`every_trust_claim_names_tests_that_exist`, `every_tool_names_its_terms_and_the_preview_matches`); `cd ui && npx vitest run src/screens/privacy-page.test.tsx` | every test named runs; preview and register match | pass | PASS | — | 2026-10-25 | agent |
| Req.3 the held conversation is bounded and never written | `cargo test -p juggler-app --lib the_conversation_is_held_in_memory_and_bounded` | only the latest kept; no writer in the driver | pass | PASS | — | 2026-10-25 | agent |
| Req.4 guide: runtimes, answering, the Beta and its gates | `docs/guide/sessions-and-the-terminal.md`; plain-words and terminology lints | clean | pass | PASS | — | 2026-10-25 | agent |
| Req.5 manifests for AJ-E2301..E2309 | `scripts/evidence_check.sh` | OK | pass | PASS | — | 2026-10-25 | agent |

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here (secret scan ran)
- [x] Paths hashed or generalised where they reveal private layout
- [x] Transcript excerpts scrubbed of personal content (none: fixtures only)
