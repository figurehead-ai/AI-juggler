# Evidence manifest — <TASK-ID>

> Copy this file into `docs/qa/evidence/<TASK-ID>/manifest.md` with the
> task's first claim. A claim missing any field is UNPROVEN (AJ-E801).

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| <req-1> | `<exact command>` | <pass threshold> | <what happened> | PASS/FAIL/UNPROVEN | <sha> | <date> | <who> |

## Binary artifacts

Binary evidence is content-addressed: `<claim>-<sha8>.png|log|json`.
Re-running a check ADDS a new file; cited artifacts are never
overwritten.

## Redaction checklist (tick before commit)

- [ ] No credentials, tokens, or keys in any file here (secret scan ran)
- [ ] Paths hashed or generalised where they reveal private layout
- [ ] Transcript excerpts scrubbed of personal content
