# Evidence manifest — AJ-E1804

Task: `docs/tasks/E18-agent-messaging.md` AJ-E1804, shared docs (P1).
Status: **DONE-EVIDENCED** on fakes (IN-REVIEW until the release PR). 2026-10-25, claude-opus.

Design: `crates/juggler-app/src/features/messages/docs.rs`, `ui/src/pages/messages/SharedDocs.tsx`; the redaction is `juggler_handover::redaction` (reused, nothing copied); D-42 point 1. Test output: [messages-tests.txt](docs/qa/evidence/AJ-E1803/messages-tests.txt), [vitest.txt](docs/qa/evidence/AJ-E1802/vitest.txt).

## What exists now

1. `share_doc` (and `juggler-agent share <name>` from stdin): text up to 64 KiB, its name one line with no folders. Redacted before it is stored, as the user chose for packs (D-14): Keep keeps what's found; Strip, Ask each time and "not chosen yet" replace it with `•••`. What was found is kept with the doc.
2. Stored content-addressed in AI Juggler's own folder: `<data>/agent/docs/<sha256>`, `0600` in `0700`; the same text twice is one file.
3. Listed per pool (20 a page) and readable by the pool's members (`read_doc`, 24 KiB a part, with its provenance note) and the user (the pool's Messages tab › Shared documents, read-only); another pool's agent is refused.
4. The withheld display uses the packs' own words (`packs::withheld_words`, `WithheldInfo`): "2 API keys withheld", or "… kept, as you chose".

## Proof

| Requirement | Proof | Result |
|---|---|---|
| Req.1 stored in AI Juggler's data folder, content-addressed, after D-14 redaction, with the user's choice shown | `messages.rs::shared_docs_are_redacted_content_addressed_and_only_the_pools` (unset → stripped; Keep → kept and said; 0600 in 0700; named by its hash) | PASS |
| Req.2 listed per pool, openable by members and the user | the same test (members read it, another pool's agent is refused); vitest "shared docs say what was withheld, as packs do, and open read-only" | PASS |
| Gate: the secret-redaction corpus | the same test's corpus: an `env-value` key, a `bearer` header, a private key block, through the packs' detector; none of the three values remain | PASS |
| Performance: redaction within the pack budget | 64 KiB through the same pass the packs use (`pack_load.rs` holds that pass's budget) | PASS |
| Live: a real agent shares a doc with a key | `docs/qa/messaging-lab.md` step 6, 2026-10-26 ([lab-2026-10-26.md](docs/qa/evidence/AJ-E1803/lab-2026-10-26.md)): Claude Code's `share_doc` of `API_KEY=sk-lab-…`; read back `API_KEY=•••`; one file named by its hash, `0600` in `0700` | PASS |
| Live: what was withheld is counted once | the same step: one key gave `secrets_found: 2`, "1 secret values" and "1 API keys" (two detectors, one span) | FAIL (the lab's finding 6) |
| Visual: the pool's docs | `_gallery/93-messages/pool-docs-{1280x800,1680x1050,2000x1250}-{dark,light}.png` | PASS |

Last updated: 2026-10-26.
