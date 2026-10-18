# Capture: claude-statusline-20261013

```text
Capture-Id:      claude-statusline-20261013
Provider:        Claude (Anthropic, Claude Code CLI)
Binary:          ~/.local/bin/claude --version → 2.1.300 (Claude Code)
OS:              macOS 15 (arm64)
Date:            2026-10-13T00:00:00-07:00 (America/Los_Angeles)
Surface:         statusline-stdin
Field paths:     rate_limits.five_hour.used_percentage,
                 rate_limits.five_hour.resets_at,
                 rate_limits.seven_day.used_percentage,
                 rate_limits.seven_day.resets_at
Raw excerpt:     (see below)
Redaction:       session_id, transcript_path, cwd, account labels and
                 model display names removed — none are part of the
                 graded claim; numbers left as observed on an
                 UNAUTHENTICATED login-screen session where the
                 rate_limits object is absent by design (see Note).
Classification:  read-only (the client pipes its own JSON to the
                 statusline script; the script's stdout is display
                 output; we write nothing)
Grade:           documented-sanctioned
Verification:    candidate — the doc leg is complete (the vendor's
                 statusline docs name rate_limits.five_hour /
                 .seven_day with used_percentage and resets_at,
                 code.claude.com/docs/en/statusline, mirrored in
                 docs/research/claude-code.md §5), but the LIVE leg is
                 not: this machine's binary is unauthenticated and
                 emits no rate_limits (see Note), so §4 path two's
                 "one capture" (a live field observation) does not
                 exist yet. Downgraded from an earlier `verified`
                 claim by the blind classification check (AJ-E601
                 pass threshold) — the gate governs.
Data class:      quota
```

## Raw excerpt (redacted) — DOCUMENTATION-DERIVED, not a live capture

The vendor's documented statusline schema shape (this is NOT live
evidence — see Note):

```json
{
  "model": { "id": "<redacted>", "display_name": "<redacted>" },
  "cost": { "total_cost_usd": 0.0000 },
  "rate_limits": {
    "five_hour": {
      "used_percentage": 42.5,
      "resets_at": "2026-10-13T18:00:00-07:00"
    },
    "seven_day": {
      "used_percentage": 12.0,
      "resets_at": "2026-10-19T00:00:00-07:00"
    }
  }
}
```

## Note — honest scope of this capture

The capture machine's `claude` 2.1.300 copy is UNAUTHENTICATED (the
gate-M1 standing re-login step): on the login screen the client does
not emit `rate_limits` ("present only for claude.ai Pro and Max
subscribers" per the docs). The excerpt above therefore pins only the
DOCUMENTED shape — zero live observations of the graded field exist on
this machine, so per §4 the verdict is `candidate`, and per §7 the
product state stays `unknown` (never `Measured`) until an
authenticated capture lands with AJ-E602.

## Verdicts

- `Claude / quota / documented-sanctioned / candidate / may-feed-product: no`
  (read-only surface; the live-observation leg lands with AJ-E602).
- `Claude / renewal / unknown` — the payload carries NO renewal or
  billing date; per D-12 the product path stays user-entered dates.
