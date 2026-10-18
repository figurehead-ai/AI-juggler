# AJ-E303 inference dogfood — this machine's real scan (2026-10-17)

The maintainer's own machine, read-only scan via the E302 engine +
E303 rules. Per-field override rate against the maintainer's ground
truth; target < 30% or the rule retunes before AJ-E304.

| Candidate (basename only) | Inferred purpose / tier | Login / tier | Activity / tier | Override? |
|---|---|---|---|---|
| `.claude` | Main / Guessed | Signed in / Found | Active / Found | purpose kept ("Main" is right) |
| `.claude-side` | Personal / Likely | Not signed in / Found | Dormant 40 d / Found | none — all three correct |

Override rate: purpose 0/2 (0%), login 0/2 (0%), activity 0/2 (0%) —
well under the 30% bar on this population. (Two real candidates; the
machine's other signal hits — rc assignments — merge into these two.)
