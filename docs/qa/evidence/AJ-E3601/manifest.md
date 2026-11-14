# Evidence manifest — AJ-E3601

Task: AJ-E3601 (the calm foundation, E36, D-117). Status: **TODO**: no
claims yet.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|

## Design reference (not evidence)

The round-2 mock the maintainer approved on 2026-11-14 (D-117), built by
hand from generic fixture names, no real accounts. Each image shows
before, round 2, and round 2 with a crashed session (L3). Opened and
checked by eye on 2026-11-14 before it was committed.

- `mock-r2-dark.png`: dark theme.
- `mock-r2-light.png`: light theme.

The proposal, research and audit behind them:
`docs/research/calm-attention/`.

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here (secret scan ran)
- [x] Paths hashed or generalised where they reveal private layout
- [x] Transcript excerpts scrubbed of personal content
