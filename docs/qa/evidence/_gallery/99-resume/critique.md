<!-- captures: a478b32437c5 -->
<!-- captures: pending -->
# 99-resume: an ended session's Resume

Design critique (D-57) against the session page header (92-session-pages). New family (AJ-E2908).

## Checklist

| Check | Finding |
|---|---|
| Grid | The header's actions: Continue in another app…, Resume (primary), Relaunch. |
| Hierarchy | Resume is the one primary action on an ended session. |
| Spacing | The header's button rhythm. |
| Icons | None. |
| Shared controls | The shared Button. |
| Disabled reasons | When it can't resume, the button says why (unit-tested; not in this preview). |
| Plain words | "Resume", "Relaunch". |
| Type sizes | The header's. |
| Both themes | Checked dark and light. |
| 1280×800 and 1024×700 | Both. |

## Good

- The ended page leads with picking up where it left off.

## Weak

- Resume and Relaunch sit side by side; the difference (same conversation vs a fresh start) is only in their tooltips.

## Fixed

- Nothing on first render.

## 0.4.1 re-score (2026-11-07, AJ-E3305)

The whole family was retaken on release/0.4.1 for the 14-line re-score (AJ-E3305, `docs/qa/0.4-walks/design.md`) and opened on contact sheets, both themes, 1280 and 1024. Shared with every screen since the last shoot: the header's New session is tinted (one filled button per view), the Floor button is a split control with its own menu, This Mac's line is its dot below 1120 px, and the rail's App headers put the profile first.

Unchanged in design: Relaunch then Resume, each with its line under it, "Ended 2:03 AM" once, This Mac on the facts line (a session's door to This Mac since 9b6b9c96b).

## 0.5.1 design critique (2026-11-24, Opus, VALIDATION §5, batch B3)

Re-captured (release-04-screens' Resume, 1280×800 and 1024×700, both themes); opened at full size: ended-page-1280x800-light. Only the rail's lens moved (F53). The ended page leads with Resume, the one filled button, with "Same conversation" under it and "Fresh conversation" under Relaunch, "Ended 2:03 AM" once in the chip, the conversation drawn as the Claude app draws it, Where it runs and Apps that can take it over beside.

- **Closed since 0.4.1:** Resume and Relaunch differ in words under each, not only in their tooltips.
- **Weak, kept:** Run in AI Juggler sits beside Resume as a fourth header action; it is rare and could live in ⋯.

| # | Line | Score | Note |
|---|---|---|---|
| 1 | Job | 4 | Pick an ended session up where it left off. |
| 2 | Prominence | 4 | Resume is the most prominent thing on the page. |
| 3 | Default | 4 | Resume is the default, Return. |
| 4 | Mode | 4 | Calm. |
| 5 | Mock-up | 4 | Matches 92-session-pages' header. |
| 6 | Areas | 4 | Header, conversation, side cards by tone. |
| 7 | Colour | 4 | Violet only on Resume. |
| 8 | Type | 4 | Captions under Relaunch and Resume in muted 12 px. |
| 9 | Actions | 4 | One filled primary named by its outcome. |
| 10 | Serious | 4 | Relaunch says it starts fresh. |
| 11 | Doors | 4 | Account, Folder, Project are doors. |
| 12 | Words | 4 | "Same conversation", "Fresh conversation". |
| 13 | States | 4 | "Its terminal output wasn't kept, so this is its conversation." said once. |
| 14 | Consistency | 4 | The session page's kit. |
| 15 | Accessibility | 4 | axe clean in the capture runs (no serious or critical); keyboard and zoom not re-walked this round. |
| 16 | Hand walk | 4 | Not re-walked by hand here; captured from the fixtures, both themes. |
| 17 | System prompts | 5 | Causes no system prompt. |
| 18 | Usability | 4 | AJ-E2908's intent (Resume first on an ended session) holds. Match. |

Lowest: 4 (every line 4 or 5).
