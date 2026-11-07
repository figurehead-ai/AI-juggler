# AJ-E3516 hand walk in AI Juggler Dev (2026-11-06)

AI Juggler Dev (debug build of `release/0.4.1` at 24d92bffc's code), on the
replica company (`HOME=/tmp/aij-replica`, `AIJ_NO_KEYCHAIN=1`, watched for a
keychain dialog: none came): 9 Apps on 3 floors (4 per floor), nothing
active, evening (Follow my clock: night). The Floor window 1278 × 798
points. Driven by synthetic mouse and key events to that pid only.

Times are from the input to the window's title naming the new view; they
include the walk tool's own pauses (about 0.46 s per click, 0.2 s per key),
so the Floor's own answer is about 0.15–0.2 s. Scores: works / speed /
clarity / layout, 1–5.

| Step | Time | Result | Scores |
|---|---|---|---|
| Open the Floor (⌘⇧M) | 427 ms to the window | Floor 1, close zoom | 5/5/5/5 |
| Lift: click "▲ FLOOR 2" | 603 ms | Floor 2; the lift now "▲ FLOOR 3 · 2 · ▼ FLOOR 1" (`dev-lift-floor2.png`) | 5/5/5/5 |
| Page Up | 340 ms | Floor 3 | 5/5/5/5 |
| Page Up at the top | — | stays on Floor 3 | 5/5/5/5 |
| Page Down ×2 | 343, 336 ms | Floor 2, Floor 1 | 5/5/5/5 |
| Page Down at the bottom | — | stays on Floor 1 | 5/5/5/5 |
| Tabs: BUILDING, FLOOR 3, 2, 1 | 631, 622, 617, 606 ms | each its view | 5/5/5/5 |
| Keys 0, 2, B | 345, 425, 417 ms | building, Floor 2, building | 5/5/5/5 |
| Building view | — | the cross-section at night: three floors of offices (all unused, dimmed), the lit lobby, the moon and stars, "FLOOR n" at each floor's left, no stray "1" (`dev-building-night.png`) | 5/–/5/5 |
| Hover an office | — | outlined, the hand cursor, "CLAUDE CODE · CLAUDE-MAX-2 / CLOSED · CLICK TO GO IN" (`dev-hover-office.png`) | 5/5/5/5 |
| Click that office | 620 ms | Floor 2 | 5/5/5/5 |
| Click the lobby | 618 ms | Floor 1 | 5/5/4/5 (the lobby leads to Floor 1; its hover says so) |
| Click the sky | — | nothing (correct) | 5/–/5/5 |
| Zoom out on Floor 1 (⌘-scroll) | — | the middle band: one name flag per area (`dev-floor1-middle-band.png`) | 4/5/3/5 |

## Found and fixed

- The middle band cut two profiles of one tool to the same "CLAUDE CODE.."
  ("claude-client-acme", "claude-client-globex"). Fixed in 24d92bffc: a
  one-word profile keeps its end ("CLAUDE CODE · ..GLOBEX"); scored 3 for
  clarity before the fix.

## Not walkable on this data

- Hover cards and office flags: nothing in the replica is active, so there
  are no people to hover and no office needs you. Their evidence is the
  offscreen tests and captures (AJ-E3517; `building-*.png` here).
- A one-floor office (no lift): the replica has three floors; the scene
  test covers it.

## Note on the walk tool

Keys first seemed not to reach the Floor; they did, but the tool's earlier
⌘⇧ presses had left ⌘ in later synthetic key events, and the Floor rightly
leaves ⌘ keys to the menu. With plain key events every key worked.
