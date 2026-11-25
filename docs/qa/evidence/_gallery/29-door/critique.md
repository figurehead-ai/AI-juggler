<!-- captures: 83db9b142e8b -->
# 29-door: the one-click door into a switch: the switch dialog from one click, the wizard's Target and What's carried steps (3- and 5-step), and the Undo toast after switching

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design passes.

## 0.3.2

The wizard steps are tidy and consistent: step chips with the current one tinted, a large title, What goes along as neutral chips and Stays behind in an amber box, and the dark dialog sits well above the dimmed page while the light one is a crisp white panel. The footer primaries ("Continue", "Switch now") are now solid violet in every wizard and oneclick-popover capture, so the main action clearly leads Back and Cancel; in dark the current step chip, the radio and the "What does this mean?" links carry the lighter violet. At 800x600 the page behind now keeps its top bar on one row: undo-toast shows the cleanup pill as a broom with "7" and drops the This Mac line, where the first pass wrapped the bar to two rows. wizard-target still stacks the same warning (the "Beta · Not proven yet" chip plus two amber boxes, "This move hasn't been proven on a real setup yet" and "Doesn't carry: Subagents' work"), which dilutes the one that matters, and in oneclick-popover "Before you switch" and "When to stop the original" are two headings stacked with nothing between. In undo-toast the user lands on a page titled just "Session" with "Unavailable · Nothing to show here yet", and at 800x600 the toast covers the footer line ("process monit…"). The 1280 rail now shows "Stopped with an error" whole. wizard-3step-carried and wizard-5step-carried differ only in the step chips. The second pass shows only behind the dialogs: the blurred terminal on the page now runs wider, to its card's edges; the dialogs themselves are unchanged.

## Weak

- undo-toast-1280x800-*.png, undo-toast-800x600-*.png: after the switch the page shows a generic "Session" title and "Unavailable"; the toast is the only sign of where the work went.
- undo-toast-800x600-dark.png, undo-toast-800x600-light.png: the toast covers the footer text.
- wizard-target-*.png: three stacked warnings on step 1.
- oneclick-popover-*.png: "Before you switch" and "When to stop the original" are two headings in a row with no content between them.

## 0.3.5

oneclick-popover re-shot (both sizes, both themes). The one-click door is the same summary as the wizard's last step: "Ready to switch to Personal", the fidelity badge, "Continue in" with Desktop (greyed: no profile on Personal yet, with why) and CLI (selected, "Same as the original"), the "Everything goes along · 5 items" chip, one line on the original, the two stop radios, and Details closed. The button reads "Switch to CLI", never "Continue here in AI Juggler".

## 0.5.1 design critique (2026-11-24, Opus, VALIDATION §5, batch B2)

Re-captured after F53, both sizes and themes, opened as dark|light pairs beside the d126 references. The dialogs are the 0.5.0 switch frame: oneclick-popover is the Confirm summary (Continue in as two choice cards, Worth knowing, What happens), wizard-target shows "Where it goes" as one target card, What happens as three fact rows and "Why it asks first" as an info Alert (the two reasons as bullets, no longer three stacked amber boxes); the 3- and 5-step What's carried differ only in their step marks. Changed since 0.5.0: only the rail behind (F53's lens fill; undo-toast's rail rows keep a readable title, F55).

| # | Line | Score | Note |
|---|---|---|---|
| 1 | Job | 4 | One click to switch, the wizard when it must ask; after it the toast's Undo. |
| 2 | Prominence | 5 | The target and what happens first; reasons last. |
| 3 | Default | 5 | The proven pair goes in one click; Next names its step ("Next: What's carried"). |
| 4 | Mode | 5 | Rich, as 28-switch. |
| 5 | Mock-up | 4 | As §8 and the kit; the target card says "Not measured" where New session says nothing for unknown room. |
| 6 | Areas | 5 | Header, target, facts, reasons. |
| 7 | Colour | 5 | The reasons in an info Alert, not amber. |
| 8 | Type | 5 | As 28-switch. |
| 9 | Actions | 5 | One primary, named. |
| 10 | Serious | 4 | "Why it asks first" says the pair is untested; the fixture says it carries "its subagents" and then "Doesn't carry: Subagents' work" (fixture, noted in 33-beta). |
| 11 | Doors | 4 | The toast's Undo; nothing on the page to open the new session. |
| 12 | Words | 4 | Plain; the fixture's contradiction above. |
| 13 | States | 4 | undo-toast: after the switch the page says "Starting on Personal…", "Switched from “Fix the flaky login test”. The original stays resumable." over a skeleton of the page's first card, until the list holds the new session (F74, 6cc4a9113); past 20 s it says plainly where the conversation is. |
| 14 | Consistency | 4 | Every toast action is the kit's secondary fill now, Undo included (F73, dda75a83f). |
| 15 | Accessibility | 4 | N1 holds; not re-walked by hand. |
| 16 | Hand walk | 4 | Preview walk. |
| 17 | System prompts | 5 | None new. |
| 18 | Usability | 4 | Undo for 10 s says the original stays resumable. |

Lowest: 3 (line 13), logged as F74; F73 for the toast's action look. No fix in this family.

**Fixed after the critique (F73, F74), re-captured (undo-toast, both sizes and themes, opened as pairs):** the page behind the toast is the new session's arriving page, its name and where it came from, a skeleton by tone where "Unavailable" stood; Undo (10s) is a neutral fill like Start another. Lowest now: 4.

## 0.5.1 final round (2026-11-24)

Re-captured once on the release HEAD after the last fixes. What changed: Behind the dialog the rail and Needs you are in their 0.5.1 style; the dialog itself is unchanged. Every state was compared with its last capture (the sweep's pixel diff, a Sonnet checker opening base, head and diff; Opus checked each suspect: none broken). Scores unchanged.
