<!-- captures: 0a5e18eb30fb -->
# 38-flows-cde: Add an account's tool cards and the New session dialog with its room-left checks

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design passes.

## 0.3.2

Add an account (c-provider-cards, c-provider-cards-unchecked) is a tidy list of tool cards, each with its status after the name and one solid violet "Claude Code →", "Fake Desktop →" or "Codex →" on the right; Gemini CLI, not installed, gets a quieter "Install Gemini CLI →" link, and the unchecked state says "Found; couldn't check its version" in plain words. The New session dialog keeps its order (how it runs, where, ranked for you, which account) and in e-launch-ok and e-launch-tight its one solid "Launch on Work Max" in the footer is clearly the action, with the "Tight — this session may use most of the room left" chip spelling out the risk. The dialog surface is a step lighter than the dimmed page in dark and white in light. Since the first pass, the lighter dark accent shows on the ai-juggler tile's ring, the "used last here" chip, the ai-juggler tag and "Launch on Work Max"; at 390 only the blurred top bar behind the dialog changed (the short broom form of the cleanup pill). The 390 problems inside the dialog are unchanged: the footer breaks, with "Launch on Work Max" wrapping onto two lines inside its button and "Launching in ai-juggler" wrapping beside it, and in e-launch-tight-390x844 the tight chip runs past the card to the dialog edge and is cut at "room le…". In e-launch-expanded the account rows' "Not measured yet ·" still leaves a dangling separator when the line wraps at 390. The second pass changed only the blurred rail rows behind the 390 dialogs; the dialogs, and their footer problems, are as they were ("Launching in ai-" / "juggler" still breaks at the hyphen). All captures were re-rendered.

## Weak

- e-launch-ok-390x844-*, e-launch-tight-390x844-*: "Launch on Work Max" wraps to two lines inside the button, and "Launching in ai-juggler" wraps next to it at the hyphen ("ai-" / "juggler").
- e-launch-tight-390x844-*: the "Tight — this session may use most of the room left" chip overflows its card and is cut at the dialog edge.
- e-launch-expanded-390x844-*, e-launch-ok-390x844-*: "Not measured yet ·" ends its line with a dangling dot.

## 0.5.1 design critique (2026-11-24, Opus, VALIDATION §5, batch B2)

Re-captured after F53 and this batch's fix, 1280×800 and 390×844, both themes. The e-launch captures are now New session's stated choice (99-new-session's sheet): "Start in ai-juggler on Claude Code CLI · Work Max — the only account used here", the folder and App lines with Change folder and Change app, Always start…, and the room verdict as a quiet line ("Tight — this session may use most of the room left"); e-launch-expanded has Change app's menu open. The 0.3.2 notes on the old Launch sheet no longer apply.

**Fixed (c3fd24f78), c-provider-cards:** Add an app had a filled primary on every installed tool ("Claude Code →", "Fake Desktop →", "Codex →"): three primaries in one sheet (BTN-1). Each is a secondary now, as the services under More already were. The desktop tool's line said "Single-account app — Juggler tracks, cannot isolate — one session at a time"; it says "One account at a time: AI Juggler can't open a second copy of this app, so one session at a time with this tool." Gate baselines `38-flows-cde/c-provider-cards-1280x800-dark` and `91-more-tools/add-account-chooser-1280x800-dark` (the same sheet) re-rendered.

| # | Line | Add an app | New session | Note |
|---|---|---|---|---|
| 1 | Job | 4 | 5 | Pick the tool; start in one click. |
| 2 | Prominence | 4 | 5 | |
| 3 | Default | 4 | 5 | One tool found skips the chooser (flow d). |
| 4 | Mode | 4 | 4 | |
| 5 | Mock-up | 4 | 4 | |
| 6 | Areas | 4 | 4 | Tool rows keep a quiet edge (action rows on the dialog's surface). |
| 7 | Colour | 4 | 4 | |
| 8 | Type | 4 | 4 | |
| 9 | Actions | 4 | 5 | After the fix: Close the sheet's only footer button, a secondary per row. |
| 10 | Serious | n/a | 4 | No-room verdicts say you can still start. |
| 11 | Doors | 4 | 4 | Install Gemini CLI → its page. |
| 12 | Words | 4 | 4 | After the fix; at 390 "~/ai-juggler ·" ends its line on the "·" and "worked here 1 min ago" wraps to three lines. |
| 13 | States | 4 | 4 | Installed, not installed, couldn't check. |
| 14 | Consistency | 4 | 4 | |
| 15 | Accessibility | 4 | 4 | |
| 16 | Hand walk | 4 | 4 | flows-cde.spec counts the primary clicks. |
| 17 | System prompts | 5 | 5 | None. |
| 18 | Usability | 4 | 4 | |

Lowest: 4 after the fix.
