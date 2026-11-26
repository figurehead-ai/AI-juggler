<!-- captures: 10db4881759e -->
# 99-turn-review: the turn review on a finished turn's feed post and under the session page's last turn

Design critique (D-57, AJ-E2814), AJ-E3820, Opus, 2026-11-26. Captured on the host (`turn-review-gallery.spec.ts`: three finished turns on Home, the first with the Beta's lines opened, a failed one opened, a clean one folded; a command-line session's page with its last turn's review opened, code-only), both themes at 1280; every capture opened. Mock-up: `~/Dev/AIJuggler-design-refs/turn-review/mock-turn-review-{dark,light}.png` (made first, the story-feed mock's style).

**The feed.** The review folds under the post's diff stat as one muted row, "Turn review · 2 files · Medium risk", the same chevron row as a turn's "Worked for 6 min" (one `FoldRow`, shared); the risk word alone carries its hue. Opened in place, it is a definition list on the post's tone behind a hairline at its left, as the mock draws it: Risk (its mark and word, then its reasons in plain words), Changed (each file in mono with its +added −removed in the diff stat's green and red, the Beta's line under it with an AI tag), Reverted (the file, then the agent's own sentence quoted), Tests ("Not run" with its amber mark, the agent's sentence), Asks you (the question). The foot says whose lines they are ("Lines by Apple's on-device model", Beta), with "Show its words" and "This is wrong" at the right. The failed turn's opens with "High · it failed · tests fail" in red, "Fail" and "What failed" quoted. The post's own bar (Mark reviewed, Open) stays where it was. Nothing else on the post moved: the post keeps its summary, the review adds the detail.

**The session page.** Under the last turn's reply, indented with the turn's words: the same row and panel, but the agent's sentences are named, not quoted again (its reply is right above): "Not run, as it says", "Asks you · Yes · its question is above", the reverted file without its sentence.

**Against the mock.** The anatomy, order, label column (88 px), colours and words are the mock's. Two differences: the feed captures scroll Home to the first post (the page header sits just above), and the session card is the page's narrower conversation column (620 px), where the panel's two columns still fit.

**Weak.** The reasons after the risk ("tests not run · asks you · part undone") repeat what the rows below say; they are the glance, the rows the detail, and they stay short. The fold row's risk word is coloured on every finished post: amber or green words on a calm surface, never a fill (PAT-COLOR-2), but a page of ten finished turns shows ten coloured words.

| # | Line | Score | Note |
|---|---|---|---|
| 1 | Hierarchy | 4 | The post's story and diff stat first; the review a quiet row under them; inside it, risk first, then what changed, then what the words said. |
| 2 | Alignment | 4 | One label column (88 px) for every row; counts right-aligned and tabular; the AI lines under their file's name. |
| 3 | Spacing | 4 | The panel's 10 px between rows, the quote's 4 px under its lead; the session page's panel indented with the turn's words. |
| 4 | Mode | 4 | Calm: tone and a hairline, no card, no border, depth on demand behind a fold. |
| 5 | Mock-up | 4 | The approved mock's anatomy, words and colours. |
| 6 | Areas | 4 | No box in a box: the panel is the post's own surface; only the diff stat keeps its inset. |
| 7 | Colour | 4 | Red high and failed, amber medium and not run and the question, green low and added lines, the accent only on the AI tag and Beta. |
| 8 | Type | 4 | 12.5 px mono for files, 13 px body for the agent's words, 12 px labels. |
| 9 | Icons | 4 | The fold's chevron; the risk mark a dot with its word (never colour alone). |
| 10 | Density | 4 | A three-row review reads at a glance; the fold keeps the feed's rhythm. |
| 11 | Motion | 4 | The chevron turns; nothing else moves. |
| 12 | Words | 4 | "Turn review", "as it says", "part undone", "Lines by …", "This is wrong"; no internal words. |
| 13 | States | 4 | Folded, opened, with and without the Beta's lines, failed, clean; hidden when there's no review (D-127, vitest). |
| 14 | Consistency | 4 | TurnWork's fold row, the diff stat's colours, the feed's quote rule, the Beta tag of the Models page. |
| 15 | Accessibility | 4 | The fold is a button with aria-expanded; the counts say "lines added/removed" to a reader; a definition list; axe clean on each capture. |
| 16 | Hand walk | 3 | Not walked in Dev yet (the coordinator's Sonnet walk follows). |
| 17 | System prompts | 4 | Nothing here asks; the Beta runs on the helper Models already set up. |
| 18 | Usability | 4 | The six answers in AJ-E3820 match the screens. |
| 19 | Rich-mode colour | 4 | A calm surface, scored as asked: status keeps its hue (risk, tests, added and removed), the AI tag and Beta the accent's soft step; nothing monochrome, nothing loud. |

Lowest: 3 (line 16).

## R4: the review's row on Settings › AI › Models (0.5.2)

Opus, 2026-11-26, AJ-E3820 R4. `models-row-{dark,light}.png`: "Each job uses" with the review built and its Beta off, captured from the Models preview's "models" state, both themes, opened.

The Turn reviews row now reads as the other Betas do (Bot's AI, Digest): its icon chip, the name with its Beta tag, "Off · Preferred over the plain review in 81 % of blind checks.", its switch off, and the model picker on Automatic, as Digest's; under the line one fainter line says which turns it covers (R5): "For Claude Code and Codex in the terminal, whose turn ends AI Juggler sees". It was "Off · Not built yet" with no switch.

**Weak.** The covers line wraps to two lines at this width; it is the row's only third line. Shorter words would lose the why ("whose turn ends AI Juggler sees").

| # | Line | Score | Note |
|---|---|---|---|
| 1 | Hierarchy | 4 | Name and Beta, then its state, then the scope, fainter. |
| 2 | Alignment | 4 | The rows' grid: chip, words, switch, picker. |
| 3 | Spacing | 4 | The covers line under the state with no extra gap. |
| 4 | Mode | 4 | Rich (Settings): chips, the Beta tag, the switch and picker. |
| 5 | Mock-up | 4 | The models-screen mock's job rows; the turn-review mock's row. |
| 6 | Areas | 4 | Rows on tone, no rules (PAT-SURF). |
| 7 | Colour | 4 | The accent's soft step on the Beta tag; the chip lights when on. |
| 8 | Type | 4 | 14 px name, 13 px state, 12 px scope. |
| 9 | Icons | 4 | The review's own chip. |
| 10 | Density | 4 | One more line on one row. |
| 11 | Motion | 4 | The switch's own. |
| 12 | Words | 4 | The section's note, plain; "in the terminal" for the command-line Apps. |
| 13 | States | 4 | Off with its switch; on reads "On · Beta with …" (vitest). |
| 14 | Consistency | 4 | Exactly Digest's row. |
| 15 | Accessibility | 4 | The switch named "Turn reviews Beta"; axe clean. |
| 16 | Hand walk | 3 | The coordinator's walk follows. |
| 17 | System prompts | 4 | None. |
| 18 | Usability | 4 | Answers 2 and 4 in AJ-E3820 hold. |
| 19 | Rich-mode colour | 4 | Its chip leads the row and lights when on; the Beta tag and picker in the kit's colours. |

Lowest: 3 (line 16).
