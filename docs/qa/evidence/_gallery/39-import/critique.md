<!-- captures: d2ad53815c61 -->
# 39-import: bringing in a Claude Code folder, its problem states, and the sign-in step after

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design fixes.

## 0.3.2

The page reads well at both sizes: a four-step tracker, a large "Bring in this folder?", a brighter lead sentence, the folder card (white on the grey canvas in light, a step lighter in dark), a Name field, then one solid violet "Bring it in" (or "Bring in settings only" in f-oversize) at the right with quieter Back and "Pick a different folder…" links. In dark the lighter accent now shows on the step-1 dot, "Bring it in", the "Sign in" button in f-signin and the Consumer Terms link; nothing else on these screens moved. f-non-config uses a red box and drops the primary, and f-oversize uses an amber box, so those two states are easy to read. The other problem states are not: in f-moved "That folder moved or is gone. Pick it again." is plain body text while the card still says "Signed in" and "Bring it in" stays the primary, and in f-duplicate the "Already imported as «Work»" line has its own Done button next to a still-primary "Bring it in", so two actions compete. In f-non-config the card still says its sign-in is kept in the macOS Keychain although the error says there is no sign-in. At 390 wide the step tracker wraps into three rows with stray connector lines, and in f-signin-390x844 the account name is cut to "cla…" while the row has room. All captures were re-rendered.

## Weak

- f-moved-*: the "moved or is gone" problem is plain text, not a warning, while "Bring it in" stays the solid primary and the card still says "Signed in".
- f-duplicate-*: a Done button beside the duplicate notice competes with the primary "Bring it in".
- f-non-config-*: the card's "Its sign-in is kept in the macOS Keychain…" contradicts the error "no settings, sign-in or session history in it".
- f-*-390x844-*: the step tracker wraps into three rows with stray connector lines.
- f-signin-390x844-*: the account name is cut to "cla…" though the row has space.
- f-oversize-*-light: the amber notice turns a pinkish beige in light, close to f-non-config's error red.

## 0.5.1 design critique (2026-11-24, Opus, VALIDATION §5)

The folder check now opens inside the setup wizard (Your apps › Add from another folder…): the wizard's frame, steps and foot around it. Opened at full size: f-moved (dark) and f-duplicate (390, light) from the 0.5.1 shoot, then f-moved (light) and f-duplicate (dark) after the fixes.

- **Before the fixes:** "Bring in this folder?" kept the standalone flow's 28 px heading and 15 px semibold lead inside the wizard, unlike every other step; f-moved said "That folder moved or is gone. Pick it again." as plain text while "Bring it in" stayed the filled primary; f-duplicate put a Done button beside the still-filled "Bring it in" (both open since 0.3.2). **Fixed** (7e858e860): the wizard's 22 px title and regular lead; moved is said in the red problem box the other states use, duplicate in a quiet box with its Done, and Bring it in leaves in both (it can only fail again). Re-rendered (gate f-normal) and re-captured (all states, both themes); checked by eye.
- **Weak, kept:** f-moved's card still says "Signed in" (read before the folder went); f-non-config's Keychain line beside its "no sign-in" error (0.3.2); the content has its own Back / Pick a different folder… row above the wizard's foot (two rows of ways out).
- **390×844 (F51), fixed** (b913d18e2): below the window's minimum (1024×680), so its twelve captures are gone and the family is captured at 1280×800 only. F50/F52 re-captured every state: the wizard's foot reads Finish later and Skip this step, the large-history line and its link say "set up a new app instead" (2c5ac5bcb), not-config says "doesn't look like a Claude Code app's folder"; checked by eye.

| # | Line | Score | Note |
|---|---|---|---|
| 1 | Job | 4 | Check a folder and bring it in, in the wizard. |
| 2 | Prominence | 4 | Card, name, then the one action. |
| 3 | Default | 4 | The name from the folder; settings-only when it's large. |
| 4 | Mode | 4 | The wizard's rich frame. |
| 5 | Mock-up | 4 | The wizard's kit; no mock of its own. |
| 6 | Areas | 4 | Card, field, problem box, actions. |
| 7 | Colour | 4 | Red only for moved and not-config, amber for oversize. |
| 8 | Type | 4 | After the fix, the wizard's title and lead. |
| 9 | Actions | 4 | One primary or none (moved, duplicate, not-config); the content's own Back row above the wizard's foot. |
| 10 | Serious | 4 | Settings-only explained; the original folder untouched, said up front. |
| 11 | Doors | 4 | Duplicate names the app it's in ("Work"); open it from Accounts is words, not a link. |
| 12 | Words | 4 | Plain, in the wizard's words (F52); the moved card's stale "Signed in". |
| 13 | States | 4 | Normal, oversize, not-config, duplicate, moved, sign-in. |
| 14 | Consistency | 4 | Problem boxes now share one look. |
| 15 | Accessibility | 4 | Problems are role=alert; not re-walked this round. |
| 16 | Hand walk | 4 | PL1 walk (Add from another folder…); not re-walked here. |
| 17 | System prompts | 5 | The folder picker is the system's own, opened by the user's click. |
| 18 | Usability | 4 | AJ-E1545 step 1 (5): reading the folder is primed by the user's own pick. Match. AJ-E1553: f-signin's one row keeps its filled Sign in. Match. |

Lowest: 4 (1280×800; no 390×844 captures since F51).
