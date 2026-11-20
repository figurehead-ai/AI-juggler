# 0.4.6 soak: the one triage (D-137)

Blocker = data loss, wrong account or action, crash, security or privacy, or a core flow broken. Everything else goes to 0.4.7 (a bug in shipped 0.4) or 0.5 (a feature or a design change), or is test or sheet work.

| # | Finding | Call | Why |
|---|---|---|---|
| SK-14 | An agent's task change that waits on you has no control where the rail and the bell lead (the task's page says "waits on your OK"; Allow is only on Home) | 0.4.7, first, with SK-10; the closest call | The approval works (rc3f passed it through Home) and nothing wrong can happen while it waits, but the doors that name it are a dead end in a core pools flow. A small, contained fix (the same AskCard on the task's page and the pool's Tasks). It fits inside the soak if main would rather ship it to Stable |
| SK-16 | The session page's "Signed in as" shows the masked email, then the Account's organization name, which for a personal Claude account is the same email in full | 0.4.7, first; main's call on the privacy rule | Nothing leaves the Mac: it's the person's own email on their own screen, so I don't read it as a privacy blocker; but the mask is a promise and screenshots get shared. One line in the Claude Code adapter (mask emails inside the organization's name) and a test |
| SK-15 | The hop limit counts per thread: a woken agent that writes in a new thread restarts the chain (a pool set to ask after 2 let 4 through) | 0.4.7, first, after a ruling (it changes D-29's unit) | Bounded by the pool's ping-pong pause (8 between two members in 10 min) and 10 a minute and 100 a day per sender, so no runaway; but the setting's words promise more. Fix: a turn woken by an agent's message carries that hop into new threads |
| SK-10 | A stopped left-behind process stays listed (and Clean up then says "already ended") | 0.4.7, first | The stop is right and safe (a second Stop is refused by the identity checks), and the list corrects at the next start. Cause found: the ledger Stop never marks its row exited, and the reconciler skips sessions with no spawn context from this run. Small fix plus a test |
| SK-12 | "New sessions in web join as" changes every session already in | 0.4.7 | Wording vs behaviour: the control sets the everyone-in-scope member's role. Bounded: a Reviewer's verdict still waits on the person's Approve. Fix the label ("Everyone in web, now and new:") and say how many it changes |
| SK-11 | After a switch, the new page shows no carried work, and the title is lost | 0.4.7 (the title), 0.5 (a carried-conversation view) | Content carrying was proven live on 0.4 (c2c); the lab's claude is unsigned. The lost title is a bug; showing the carry is design |
| SK-1 | A pool's Delete has no visible door (the card's ⋯ shows only on hover; Delete at the panel's foot) | known: AJ-E3014 item 6 (queued by rel-05) | Add to that item: the card's ⋯ works (Opus deleted a pool with it) but is invisible in every picture; show it always, or put Delete in the pool's header |
| SK-13 | A named session's role, and a pool's edge rules, are set only on canvas cards and lines (LA, LA2 step 3) | 0.4.7 | The List view (the canvas's accessible twin) needs the role control; the template promises a Reviewer the wizard can't name |
| SK-2 | "What this affects" greys Continue with no reason | 0.4.7 | D-127: say why, and agree with Home's card |
| SK-3 | A new pool's diagram packed against the right edge | 0.4.7 | Layout bug |
| SK-4 | The date field's placeholder is cut ("Choose a da") | 0.4.7 | Port 0.5's 1d4e7c7d5 (one minimum width in DateField) |
| SK-5 | An errored session's page doesn't say why | 0.4.7 | Show the exit code and when, where known |
| SK-6 | "Suggested" stays on an App that's out of room | 0.4.7 | The suggestion contradicts its own warning |
| SK-7 | No "Use a template" door once the first member joins | 0.4.7 | A pool still being set up should keep it |
| SK-8 | A pool's Tasks panel is empty for ~7 s | 0.4.7 | Instant and paged: a skeleton or cached rows |
| SK-9 | A Pools card names an Account member "An Account" | 0.4.7 | The card's name lookup misses Account members |
| P-1 | Needs-you rows never quote the ask | 0.5 (product), 0.4.7 (lab) | A real CLI session shows its prompt in the terminal; quoting the notice text is new design (with the home feed) |
| P-2 | "Plenty of room" beside 72 % CPU never says what room measures | 0.4.7 | Wording on This Mac |
| P-3 | No "start these on whichever account has room" | 0.5 | A feature |
| LA3 | A refusal "This pool doesn't let you message that member." doesn't name its rule | 0.4.7 | The agent guessed a wrong cause; say "Member to Member is blocked in this pool" |
| P-4 | Home's headline vs the question the person asked | 0.5 | The home feed design (0.5) |

Already queued, seen again in the soak (no new item): the wizard's left rail can't be pressed (item 12, P19); the Stop toast names the folder (item 10, LA); New session suggests another project and App first (item 9, LA2 and LA3, every start); the Pools cards move (item 1, LA).

Test and sheet work for 0.4.7: a lab's own web storage (all Dev builds share ~/Library/WebKit/ai.figurehead.juggler.dev); K3 staging a readable ask; PF1 with running sessions; a "back after a reset" seed; N5 with both repos; sheet wording (N1, N3, P10, P11 step 2); the seeded "AI Juggler restarted" modal in A/K labs; a sweep of 261 + 266 `juggler-app-<n>` test-build folders in ~/Library/WebKit and ~/Library/Caches (dry-run list first).

**No blocker.** Stable on 2026-11-27 holds. The closest calls, for main: SK-14, SK-15 and SK-16 are small and contained; if main would rather ship any of them in Stable, each fits inside the soak (a blocker fix needs 3 days on beta, so one on beta by 2026-11-24 keeps 2026-11-27).

Walk results not yet passing: P2 step 3 and P1 step 9's Remove (canvas-only, the driver can't press them); P7 step 3 (the prompt named AI Juggler's label, which Claude's ListAgents doesn't show; rc3f proved it live); P11's pause (SK-15); P1 step 5's approve (walker timing); M5 (no seed).
