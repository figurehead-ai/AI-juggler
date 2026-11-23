# Evidence manifest — AJ-E1553

Task: `docs/tasks/E15-spec-gaps.md` AJ-E1553, a sign-in finishes itself:
AI Juggler sees it succeed, says so and clears its entry; one that ends
early says so and keeps its last lines (P1). Built with AJ-E1545 R8's
sign-in card. Status: **DONE (0.5.1)**, 2026-11-23.

Commits: 00a379508 (each adapter reads its tool's sign-in page and code,
and a "signed in?" signal), c6afbf9c2 (`features/sign_ins`: the watch,
the outcome, the restart clean-up), 252f1b311 (`SignInCard`,
`SignInPanel`, the wizard's row, the rail), a0697d024 (clippy),
06a3162dc (start time as a safe window number), f6187543f, 85d1f8cf2 and
9e963d0d7 (the card in every state, status regions, the window's list).

No captures: the card's states are proved by `sign-in-card.test.tsx`
against the built components (generic fixtures, `alice` only).

## Requirements

| R | Claim | Verdict | Proof |
|---|---|---|---|
| R1 | While a sign-in runs AI Juggler watches its adapter's "signed in?" signal (file events plus the adapter's check, metadata only, never a credential's contents); on success it ends the sign-in, says "✓ <tool> is signed in", the Account and App lists are read again by themselves, the rail's entry leaves and the wizard's row ticks | PASS | `sign_in_finish.rs` (8 tests: every adapter's success detection and early exit); `sign_ins::watch` `a_sign_in_comes_to_signed_in_or_ended_with_its_last_lines`, `a_page_is_told_once_and_its_code_kept`; `sign-in-card.test.tsx` "says ✓ signed in, with Done", "a sign-in that finishes leaves the rail by itself and flips the lists once"; the wizard row reads the same list (`FirstRun.tsx` `signIn?.state === "signed_in"`) |
| R2 | A process that ends without success leaves "Sign-in ended before it finished" with its last lines kept and Try again; never an empty terminal | PASS | `sign_ins::watch` `only_the_tail_of_a_long_output_is_kept`; `sign-in-card.test.tsx` "shows an ended sign-in's last lines and Try again, never an empty terminal", "says so when an ended sign-in printed nothing", SignInPanel "Try again starts a new sign-in for the same App and opens it" |
| R3 | On restart, sign-in entries whose Account is signed in (or gone) are cleared | PASS | `sign_ins` `a_restart_clears_the_sign_ins_whose_app_is_signed_in_or_gone`, `only_ended_sign_ins_are_kept_over_a_restart` |
| R4 | The "check again from Accounts" words go | PASS | `sign-in-footer.test.tsx` "names the App's own tool and never asks to check again from Accounts"; no such words left in `ui/src` outside that test and a doc comment |
| R5 | Through the adapters' sign-in hooks only, no per-tool code outside them | PASS | the page, code and signal come from `juggler-adapter::sign_in_page` and each adapter's hook (00a379508); `features/sign_ins` names no tool; `no_bare_tools` (4 tests) passes |

Checks run 2026-11-23 on HEAD 9e963d0d7 plus the tree: clippy `-p
juggler-app -p juggler-adapters --all-targets -D warnings` clean;
`sign_in_finish` 8/8; `--lib sign_ins` 9/9; `no_bare_tools` 4/4;
vitest `sign-in-card`, `first-run`, `first-run-pairing`,
`sign-in-footer` pass; tsc clean; `ui_lint` clean for these files;
`terminology_lint.sh` OK; `private_data_lint.py` OK.

## Usability reasoning, re-checked on the built UI

1. Someone signing in from the wizard or the rail expects AI Juggler to
   notice: matches (the window's one sign-in list turns the card, the rail
   and the wizard's row together, no re-check asked).
2. "✓ <tool> is signed in" the moment it is; an early end says "Sign-in
   ended before it finished" with its last lines and Try again: matches
   (both lines as status regions; Try again is the one primary).
3. One detection for the wizard and the rail, as AJ-E1545's card: matches
   (`useSignIns` feeds `SignInCard` in both and `RailSignIns`).
4. A finished or stale entry leaves the rail by itself, also after a
   restart: matches (signed-in rows drop from the rail; the restart clears
   signed-in and gone ones, keeps ended ones with their outcome).
5. No new system prompt: matches (opening the tool's page raises none,
   `docs/design/os-prompts.md`).
6. An empty terminal and "check again from Accounts" read as broken:
   matches (an ended sign-in shows its lines or "<tool> said nothing
   before it ended", never a blank terminal; the words are gone).

## Left

- The persona walk (sign in through the wizard, see the rail's entry
  leave) is the Sonnet QA pass's, on the next RC.
