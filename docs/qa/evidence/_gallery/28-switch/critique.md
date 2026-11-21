<!-- captures: e46fc7ce5979 -->
# 28-switch: switching a conversation to another account: the "Continue on another account" list, the 3-step switch dialog's Confirm step in its variants, the running and failed states, and the notes and switch-anyway prompts

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design passes.

## 0.3.2

The Confirm step has a sound reading order in both themes: step chips, a large "Ready to switch to Personal" title, a short bullet summary, an amber Before you switch notice, then the stop-the-original radios with "recommended" in green, and the dialog panel stands well off the dimmed page in dark. The footer primaries ("Switch now", "Retry", "Switch anyway") are now solid violet, so the main action clearly outweighs Back and Cancel in every Confirm, notes, switch-anyway and failed capture; in dark the radio, checkbox and Confirm chip carry the lighter violet. At 800x600 the page behind the dialogs now keeps its top bar on one row, where the first pass had it wrapping. The long variants still clip: confirm-account-steps and confirm-set-up at 1280x800 end mid-section at the scroll edge (in confirm-set-up the github MCP row is sliced), confirm-token-steps opens scrolled so the title is gone (at 1280x800 now also the "Personal doesn't have 2 things" line, so release-notes sits straight under the step chips) and at 800x600 the github row's own heading is above the top edge. After the second pass the token command wraps at spaces ("--scope user", "user github" no longer split mid-word), but it still breaks at hyphens: "github --" / "scope user" and "server-" / "github"]" (the "-y" stays whole here); dialog-stop-800x600 cuts the selected option's hint under the footer. targets-out-of-room shows the "Continue on another account" dialog over the session title (cut to "feat/"), with no dimmed backdrop in dark, and every option carries the same amber Beta chip, so the warning colour stops meaning anything. failed says "resume it on Work where you left it" (the account is Work Max) yet offers only Cancel and Retry, and executing lists five steps with no mark of which one is running. In the other captures only the dimmed page behind changed: its terminal's black now reaches the card edge.

## Weak

- confirm-token-steps-800x600-dark.png, confirm-token-steps-800x600-light.png: opens scrolled; the github row's checkbox and name are above the visible area.
- confirm-token-steps-*.png: the command block still breaks at hyphens ("github --" / "scope user", "server-" / "github"), so the flag and the package name read as split; mid-word wraps are fixed.
- confirm-set-up-1280x800-*.png, confirm-account-steps-1280x800-*.png: the last set-up row is cut at the scroll edge.
- dialog-stop-800x600-*.png: the selected "Stop it now and switch" hint is cut by the footer.
- confirm-set-up-800x600-*.png: the set-up section the capture is named for is entirely below the fold.
- targets-out-of-room-1280x900-*.png: no backdrop in dark; the dialog covers the session title; the same amber Beta chip on all three accounts; in Codex and Personal "Not enough use to forecast" sits right-aligned on its own line, leaving a gap.
- failed-1280x800-*.png, failed-800x600-*.png: "resume it on Work" names the account wrongly, and there is no resume button.
- executing-*.png: no current-step or done marks on the five steps.
- notes-conflict-1280x900-*.png: "Before you switch" and "Set up on Personal" are two headings stacked with nothing between them.

## 0.3.5

Re-shot: dialog-finish, dialog-stop, executing, failed, notes-*, switch-anyway, targets-out-of-room (both sizes and themes). The confirm step now opens on "Continue in" with Desktop and CLI cards. On these captures Personal has no Claude app profile, so Desktop is greyed with why ("No desktop app profile on Personal yet…") and CLI is selected, tagged "Same as the original". Under them sits one coloured summary chip ("3 go along · 1 stays behind" amber, or "Everything goes along · 5 items" green) and one line on the original. The stop radios are one line each, with the examples behind "What does this mean?". The buttons read "Switch to CLI", or "Switch to CLI anyway" when Personal is out of room. At 800×600 the body scrolls under a fixed footer (Cancel, Back, Switch to CLI). executing lists its stages, the fourth saying where it starts: "Starting it in a terminal on Personal" (on Desktop: "Opening it in the Claude app on Personal"); failed is unchanged apart from the pills.

### Weak (0.3.5)

- At 800×600 the out-of-room note sits below the fold; the amber "Personal is out of room: see below" line above it points there.

## 0.5.0 (D-142): the switch wizard rebuilt to the design system

Reviewed side by side with the d126 kit and review references (`kit-*`, `start-*`, `review-*`, `running-*`), both themes, at 1024×680 and 1280×800, every step walked in the preview (Playwright, the gallery's fixtures), not yet in the Dev app.

What changed, step by step:
- **Continue in another app (the target choice).** Each App is a card row: its own logo at 40 px with the profile's badge beside it, the App's name in ink, "On <account> · forecast" under it, then tags: the room in its status colour (88% left green, 8% left amber, Out red), one accent Suggested, how much carries as a neutral tag ("Full story, fresh start · Beta", no more amber hatching on every row). The Suggested card wears the accent's edge; a chevron says the card opens the switch. The same card in the terminal list and the desktop one.
- **Header, every step and phase.** A 40 px icon tile, "Switch to Personal" (or "Continue on Claude app · Personal" for Move), "Fix the checkout flow · now on Work", then the from → to strip with the destination App's logo (it follows Desktop or CLI), then the step marks: "STEP 2 OF 3: WHAT'S CARRIED" in the accent, numbered dots joined by a line, done ones ticked on the accent's tint, the current one filled.
- **Target.** "Where it goes": the App card (logo, name, account, room and badge tags), then "What happens" as fact rows on the tinted panel (Carries, Original, Room, Where), the out-of-room or low room as a needs-you Alert with "What does this mean?", the wizard's reasons as an info Alert ("Why it asks first"), never amber boxes.
- **What's carried.** A tinted headline ("3 go along · 1 stays behind", ✓ or ! chip), then one card of rows: the kind's icon in a 30 px chip, the thing in ink, its line in ink-2, a tag at the right (Goes along green, Recreated and Stays behind neutral, Not on Personal yet amber). The keys question, when it applies, as two choice cards (Hide them · Recommended, Leave them in) with a needs-you Alert saying what was found, and CheckRows.
- **Confirm.** "Ready to switch to Personal", Continue in as two choice cards with the tool's logo (Desktop greyed with its reason on its face), then the Alerts (room, disk short, the check's warnings as one info Alert "Worth knowing"), then What happens as fact rows (Copies with the badge tag, Carries opening to the list, Original, Disk space, Runs by itself, Room, Needs you), the stop choice as two cards (Let it finish · Recommended), set-up under "Set up first", Details folded.
- **Move or Copy.** §8 as built, its summary panel now the shared tinted one; its tags keep an edge so they show on the chosen card; each conversation row has a status icon (moves, already there).
- **Progress.** "Switching to Personal…", a tinted panel with the stage it's at, "Step 2 of 5" and a bar, then the five steps (✓ done, spinner now, ring next). Move's is the same shape: the stage's words, its percent and bar, then Quit both apps · Move the conversation · Reopen both.
- **Done.** A success tile, "Switched to Personal", the facts (Where with the App's logo, Original, its sessions, what runs by itself), Close (ghost) and the one primary. Moved: the four outcomes as fact rows, each with its ✓, ! or clock icon, Undo the move as a secondary.
- **Failure.** "The switch didn't complete" over a broken Alert with the failure's own words; a refusal that changed nothing is "Nothing was moved" over an info Alert. Copy details and Report a bug… are kit buttons with icons. Esc, × and Cancel close both.
- **Footers.** Next names its step ("Next: What's carried"); Switch back, the cross-Mac switch back and the pairing wizard now draw their buttons in the frame's fixed footer.

### Scores (VALIDATION §5, 1–5)

| # | Line | Targets | Target | Carried | Confirm | Move/Copy | Progress | Done | Failure |
|---|---|---|---|---|---|---|---|---|---|
| 1 | Job | 5 | 4 | 5 | 5 | 5 | 4 | 5 | 4 |
| 2 | Prominence | 5 | 4 | 5 | 4 | 5 | 5 | 5 | 5 |
| 3 | Default and folding | 5 | 4 | 4 | 5 | 5 | n/a (no choice) | 5 | 5 |
| 4 | Mode (rich) | 5 | 5 | 5 | 5 | 5 | 4 | 4 | 4 |
| 5 | Matches the mock-up | 4 | 4 | 4 | 4 | 5 | 4 | 4 | 4 |
| 6 | Areas and spacing | 4 | 5 | 5 | 4 | 4 | 5 | 5 | 5 |
| 7 | Colour | 4 | 5 | 5 | 5 | 5 | 5 | 5 | 5 |
| 8 | Type and hierarchy | 5 | 5 | 5 | 4 | 5 | 5 | 5 | 5 |
| 9 | Actions | 5 | 5 | 5 | 5 | 5 | 5 | 5 | 5 |
| 10 | Serious choices | 4 | 4 | 5 | 5 | 5 | n/a | 5 | 4 |
| 11 | Doors | 4 | 4 | 4 | 4 | 4 | 4 | 5 | 4 |
| 12 | Words | 4 | 5 | 5 | 4 | 5 | 5 | 5 | 5 |
| 13 | States and speed | 4 | 4 | 4 | 4 | 4 | 5 | 5 | 5 |
| 14 | Consistency | 5 | 5 | 5 | 5 | 5 | 5 | 5 | 5 |
| 15 | Accessibility | 4 | 4 | 4 | 4 | 4 | 4 | 4 | 4 |
| 16 | Hand walk | 4 | 4 | 4 | 4 | 4 | 4 | 4 | 4 |

Dialog add-on: VALIDATION N1 (`tests/visual/dialog-fit.spec.ts`) passes for Move or Copy, the wizard's three steps, done, Continue on another account and Add a machine, at 1024×680 and 1280×800 in both themes: header and footer fixed, the body alone scrolls, the primary visible at every scroll position, the first choice group whole on open.

### Fix round (D-137)

Changed after the coordinator's look: the target list's blank band under the dialog header is gone (the list's top margin only in the card, the body's top padding 4 px); each target card has at most two tags (its room, Suggested), how much carries a quiet line under the name, and Doesn't carry's items in lower case ("subagents' work, the task list"); What's carried groups its rows under "Goes along · 3" and "Stays behind · 1" with no tag per row, its summary chip a neutral (i) (a fact, not amber); every choice card in the switch (Move or Copy, Continue in, keys, when to stop, the cross-Mac ones) has its logo top-aligned with its title (`RadioCard align="start"`); Move's two tags sit side by side; the header's close draws its 24 px cross (the button's padding had shrunk it to 8 px); a refusal under "Nothing was moved" no longer says "so nothing was moved" again; the switch to a paired Mac has the same header (icon tile, title, This computer → Team on Mac mini) and its sending, done and failed bodies the same stage panel, success summary and broken Alert.

Re-scored lines (the rest stand): Targets 5 Matches the mock-up 4→5, 6 Areas 4→5, 7 Colour 4→5, 12 Words 4→5; Carried 5 Matches the mock-up 4→5, 12 Words stays 5; Move/Copy 6 Areas 4→5; Confirm 6 Areas 4→5; Failure 12 Words 5 (no longer says it twice). N1 passes again for every switch dialog at 1024×680 and 1280×800, both themes.

Why not 5 still:
- Targets: the dialog is the S size, so at 1024×680 two cards show before a scroll.
- Target and Confirm: the header's "from" side shows an account icon, not the source App's logo (the check doesn't name the source App).
- Accessibility and the hand walk: checked by the unit tests and the preview walk; Reduce Motion, Increase Contrast and 200% not re-walked by hand, and the Dev app not walked yet.

### Weak (0.5.0)

- A step heading that takes focus after a keyboard press shows the global focus ring (50-cross-switch progress-1280x800-dark: the ring around "Switching to Team on Mac mini…"): the heading's `outline-none` loses to the unlayered `:focus-visible` rule.
