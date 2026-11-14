# Evidence manifest — AJ-E3701

Task: AJ-E3701 (Home in two columns, E37, D-118). Status: **TODO**: no
claims yet.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|

## Design reference (not evidence)

The Home feed mock the maintainer approved on 2026-11-14 (D-118), rendered
from `docs/research/home-feed/mock-home-feed.html` with generic fixture
names only. Each image shows the wide layout (1280 px window) and the
stacked narrow one (about 900 px). Opened and checked by eye on
2026-11-14 before it was committed.

- `mock-home-feed-dark.png`: dark theme.
- `mock-home-feed-light.png`: light theme.

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here (secret scan ran)
- [x] Paths hashed or generalised where they reveal private layout
- [x] Transcript excerpts scrubbed of personal content
