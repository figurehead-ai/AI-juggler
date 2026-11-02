<!-- captures: 185b05a3b585 -->
# 99-conversation-feed: a Claude session's conversation, followed live

Design critique (D-57, AJ-E2814) for 0.3.5 (ADR-0024), against the Claude app's Code tab (`docs/research/native-conversation-display.md` §1) and the reference screens of 92-session-pages. The captures are the Conversation card alone, at 1280×800, both themes.

## What each capture shows

- **stretches**: a Claude turn drawn as the Claude app draws it: the narration line, then each run of work as one line ("Ran the checkout test 40 times", "Read 2 files, searched code", opened onto its three steps), the texts in full between them. No "Worked for" row.
- **running**: opened at the bottom; the person's mid-turn message as their bubble, then the turn still working with its current run in the "-ing" form ("Running the unit tests") and a pulsing dot.
- **new-messages**: the reader scrolled to the top while the reply arrived: the view did not move, and a "New messages ↓" pill sits over the bottom of the list.
- **banner**: the latest changes couldn't be read: one line under the header says so; the turns stay, and the newest is still in view.

## Checklist

| Check | Finding |
|---|---|
| Grid | Turns on the card's 760 px reading column; fold lines indented with the agent's text under its name. |
| Hierarchy | Words in ink at body size; fold lines muted at meta size, so the text reads first, as in the Claude app. |
| Spacing | 12 px between a turn's parts, 24 px between turns, unchanged from 0.3.4. |
| Icons | The fold's chevron is the one the 0.3.4 work row used; the pill's arrow is a plain down arrow. |
| Shared controls | Fold lines are buttons (focus ring, `aria-expanded`); the pill is a button; no system controls. |
| Disabled reasons | "Show earlier messages" is disabled only while a page loads or the follow restarts. |
| Plain words | "New messages", "Couldn't read the latest changes. Trying again.", "Running the unit tests". |
| Type sizes | Text 14 px, fold lines and banner 13 px. |
| Both themes | Checked dark and light; the running dot is the live colour in both. |
| 1280×800 and 1024×700 | 1280×800 only: the card is the same column at 1024 (92-session-pages covers the page at both). |

## Good

- A Claude turn reads like the app: the words, with the work between them one quiet line each.
- New words never move a reader who scrolled up; the pill is the only change on screen.
- A problem reading is a line over the turns, not an empty card.

## Weak

- The pill covers a line of text while it shows (it floats over the list, as the apps' own do).
- The banner is neutral grey; "Trying again" problems and "it ended" notes look the same.
