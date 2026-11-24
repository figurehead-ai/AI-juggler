<!-- captures: 55d1da45e3ad -->
# 66-first-run: the first-run steps (bring in accounts, sign in, computer access, Figurehead account) and Home right after them

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design fixes.

## 0.3.2

The flow keeps a clean shape: stepper, large heading, content, and one primary bottom-right in solid violet. After the design fixes the dark accent is lighter, and it shows on the current step's number in the stepper (access, access-reopen, after-bring-in-copies, figurehead), the checkboxes and the selected radio card's ring in found and found-copies, the terms links in one-failed and where-we-looked, and the logo mark and the rail's New session outline on home-after-first-run, whose top bar stays one row. The buttons themselves did not change. The intro under each heading is still near-white and semibold, almost the heading's weight; the 0.3.1 grey regular weight sat better. Some screens still stack equal primaries: access-reopen has two solid violet buttons (Reopen AI Juggler, Continue), after-bring-in-copies has six violet Sign in buttons above "Sign in to all 6, one after another", and figurehead gives its two choices the same neutral weight. access-one-allowed still says "All set." above the "Allow… Skip any of them" paragraph, with no step marked done or current. In found-copies and one-failed the profile paths are cut mid-folder ("ClaudeCodeProfiles/wor…"), and one-failed and where-we-looked still open with the list's top cut, starting at the work-c row.

## Weak

- access-one-allowed-1280x800-dark: "All set." keeps the "Allow… Skip any of them" paragraph, and the stepper marks no step done or current.
- access-reopen-*, access-reopen-live-sessions-*: two solid violet buttons (Reopen AI Juggler, Continue) compete.
- after-bring-in-copies-1280x800-dark: six violet Sign in buttons plus the main button make a wall of primaries.
- All: the intro paragraph is bright and semibold, close to the heading's weight.
- found-copies-1280x800-dark, one-failed-*: profile paths are ellipsised mid-folder; one-failed and where-we-looked open with the list's top cut (first visible row is work-c, the profiles header is off screen).

## 0.4.1 rc31 (2026-11-10)

figurehead-1280x800-dark is removed: first run has three steps since D-97 2 (4049d6c56), Bring in your apps, Sign in where needed and Computer access, and no longer captures a Figurehead account step (Settings offers it). The other captures here were taken before D-97 and are not retaken in this round; the gate's own baselines (`ui/tests/visual/__baselines__/linux/gallery/66-first-run/`) show three.

## 0.4.2 (2026-11-12): the fixture renamed

Only the captures that showed the old fixture folder name were re-shot: the copy, its menu header and its row now read "acme-max". Nothing else on these screens moved; the critique above stands.

## 0.5.1 design critique (2026-11-24, Opus, VALIDATION §5)

The first run is now the setup wizard (AJ-E1545): a full-window sheet with the app's mark and "Set up AI Juggler", the steps beside (Your apps, Sign in, Mac access, Meet Bot, Keep running, each with its value once set), the step's eyebrow and title, and a fixed foot. Opened at full size: found, found-copies, where-we-looked, one-failed (light), after-bring-in, after-bring-in-copies, access-one-allowed, access-resumed (light) and home-after-first-run (the Welcome, still), then after-bring-in-copies, access-one-allowed and access-resumed (both themes) after the fixes. Reference: the calm round-2 mock (one filled button per surface; the permission card's Allow quiet) and the d126 kit's wizard (the pool setup's FlowLayout, which this shares).

- **found:** "Which AI apps do you use?", one row per app with its Terminal or Desktop chip and plain sign-in words, the running desktop profiles unticked (D-84), "Bring in 5 apps" matching the ticks, Where we looked and Providers' terms closed. Good; F45's foot holds one row.
- **after-bring-in-copies (Sign in), before the fixes:** six filled violet Sign in buttons plus "Sign in to all 6, one after another" (seven primaries, flagged since 0.3.2), under a 28 px heading and a 15 px semibold lead unlike every other wizard step. **Fixed** (7e858e860): with several left each row's Sign in is secondary and "Sign in to all 6" is the one primary; the step takes the wizard's 22 px title and regular lead. Re-captured and checked: one filled button.
- **access-one-allowed / access-resumed:** the allowed card still said "macOS will ask you · optional · change it later…" after macOS had asked. **Fixed** (7e858e860): "Change it in System Settings › Privacy & Security". Re-rendered (gate access-one-allowed) and re-captured (access-resumed both themes); checked by eye. The card that still asks keeps the dialog's picture, Click and If you say no (AJ-E1547), with Allow tinted, not filled, and Next: Meet Bot the one primary.
- **Foot (F50), fixed** (b913d18e2): the foot reads Finish later (leave setup; it reopens where it was left), the step's links, Skip this step, then the step's one primary last at the right end (FlowFoot's end slot: "Bring in 5 apps", "Sign in to all 6, one after another"). Sign in has no in-page "Sign in later" inside the wizard (the foot's Skip this step does it); a row's own reads "Sign in to this app later". Re-rendered (gate found, where-we-looked, after-bring-in, access-one-allowed) and re-captured (every state); checked by eye: found and found-copies keep one row at 1280 with the names note, and the 1024 regression cells keep one row without it (F45).
- **Fixtures (F51), fixed** (b913d18e2): every journey state on the pinned clock; found-copies and one-failed now read "Used 2 days ago", "Used yesterday", "Used 3 days ago", "In use now", like a real Mac.
- **access-resumed:** a resumed wizard shows the done steps without their values (Your apps, Sign in have none after a reopen); WIZ-1 wants each step's value. Weak.

| # | Line | Score | Note |
|---|---|---|---|
| 1 | Job | 4 | Apps, sign-ins, Mac access, Bot and keep running in one place; Meet Bot and Keep running have no capture in this family. |
| 2 | Prominence | 4 | The rows first; More options and Where we looked folded. |
| 3 | Default | 4 | Signed-in folders ticked, desktop profiles not (D-84); "Use them where they are" Recommended. |
| 4 | Mode | 4 | Rich wizard per D-126. |
| 5 | Mock-up | 4 | Built on the pool wizard's kit; the permission card as the mock's (Allow quiet). |
| 6 | Areas | 4 | Steps, page, foot; the permission card nests its dialog picture in a panel (a picture, not a box of controls). |
| 7 | Colour | 4 | Accent on the step, the primary and "Set up a new app instead"; green only for Allowed and Brought in, red for Couldn't bring in. |
| 8 | Type | 4 | After the fix every step's title and lead match; the embedded steps' title sits ~18 px lower under the eyebrow than Mac access's. |
| 9 | Actions | 4 | One filled primary per view, last at the foot's right end (Bring in N, Sign in to all N, Next: …); secondary to its left; the exits named by what they do (Finish later, Skip this step, Sign in to this app later). one-failed keeps its Continue with 5 apps in the outcome card, last in that card. |
| 10 | Serious | 4 | Make separate copies says each copy needs a sign-in; Providers' terms closed (AJ-E1548 R11). |
| 11 | Doors | 4 | Details per row; each step links where it lives later (Settings › Privacy & Security). |
| 12 | Words | 4 | Plain; "Set up a new app instead", "Add from another folder…", times on the capture clock (F51). |
| 13 | States | 4 | Found, copies, one failed, allowed, resumed; F48 kept Sign in on its step. |
| 14 | Consistency | 4 | FlowLayout, the shared rows and chips. |
| 15 | Accessibility | 4 | Focus on each step's title, Tab kept inside (a11y.spec, F47); not re-walked this round. |
| 16 | Hand walk | 4 | PL1 walk and re-walk on the real app (AJ-E2323: F20–F48); not re-walked here. |
| 17 | System prompts | 5 | Each Mac access card shows macOS' own dialog, what to click and what a no means, before Allow; after the fix the allowed card says where to change it. |
| 18 | Usability | 4 | AJ-E1545 / E1553 / E1547 (below). |

**AJ-E1545 match:** (1) "Step 1 of 5" says how long; the header's mark is the app's icon. Match. (2) Each step says what and why before its controls; each permission card shows the dialog and the button to click. Match (Meet Bot's AI names and size are not in these captures). (3) Setup in one place, each step pointing to where it lives later. Match. (4) Skip this step on every step; Finish later leaves and the wizard reopens where it was left (access-resumed); each exit says what it does (F50). Match. (5) Accessibility, Screen Recording primed on their cards. Match. Step 1 (1)–(6): one row per app, Terminal/Desktop chips, plain status words, desktop rows not pre-ticked, "Bring in N" equals the ticks, terms a closed fold. Match. Step 2: the rows open each tool's own sign-in page ("AI Juggler never sees your password"); the code-and-browser card is not captured here. Partial (not visible).
**AJ-E1553 match:** the finished and ended sign-in states are not in this family (40-login and the unit tests cover them); after-bring-in shows "Sign in · None needed" in the steps. Not visible here.
**AJ-E1547 match:** no system dialog appears in these captures without its card first. Match.

Re-scored after F50–F52 (2026-11-24, Opus): every capture opened again at full size (found, found-copies, where-we-looked, after-bring-in, after-bring-in-copies, access-one-allowed, access-resumed both themes, one-failed both themes, home-after-first-run). Lowest: 4.
