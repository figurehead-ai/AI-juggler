# Evidence manifest — AJ-E2109

Task: `docs/tasks/E21-desktop-first.md` AJ-E2109, Accounts are
subscriptions; profiles sit under them (D-35). Built in 0.1.3 (ac0e41dc0);
re-audited against the code on `release/0.4.1` (HEAD 051e71e4b),
2026-11-07. Since then D-62 (AJ-E3201/E3202) names each row an **App**
("Claude Code · Work") and stores each sign-in as its Account with its
plan, renewal and fee; the 0.4 design review (50326bbd5) took the folder
off the row and said "Not measured yet" once.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| R1 profiles grouped by signed-in identity (account and organisation), worked out when read; no database change in 0.1.x | cargo (runs in CI): `juggler-app` `src/features/subscriptions/mod.rs::two_profiles_on_one_login_are_one_account`, `::a_personal_plan_and_a_team_seat_on_one_email_stay_apart`, `::one_login_on_two_tools_is_two_accounts`, `::not_signed_in_comes_last_and_an_unreadable_sign_in_stands_alone`, `::the_key_never_carries_the_vendor_ids`; `juggler-adapters` `tests/claude_identity.rs::a_personal_plan_and_a_team_seat_on_one_login_name_their_organisations`; `juggler-app` `src/identity.rs::the_verdict_never_passes_a_different_identity`; `cd ui && pnpm -s exec vitest run src/screens/accounts/accounts-grouped.test.tsx`: "two subscriptions over six profiles: two accounts, each with its profiles, Not signed in last" | one account per tool + account id + organisation; personal vs team seat apart | the cargo tests exist at those names; vitest passes (Acme, Claude Max, Not signed in). Grouping is still worked out on read from `probe_identity` (`subscriptions/mod.rs` header); 0.1.x had no schema change, and the stored Account came later, by D-62 in 0.4 | PASS | ac0e41dc0 | 2026-11-07 | Claude (self) |
| R2 an account card shows name, email, plan, and its limits, or "Not measured yet" with the way to measure | `cd ui && pnpm -s exec vitest run src/screens/accounts/accounts-grouped.test.tsx`: "two subscriptions over six profiles: …" (`account-name`, `account-line` "o…@work.com · Team"), "one profile: one account; nothing measured says so once, in one line, and still shows its usage" ("Not measured yet ▸·Measure its limits…", `account-measure`), "a measured account keeps its dial, and says no unmeasured line", "names an account by the name the backend gives it (D-62), never a folder's"; `screens/accounts/AccountCard.tsx`; gallery `_gallery/93-accounts-profiles/accounts-six-profiles-1280x800-dark.png` (looked at) | name, email · plan, dials or one "Not measured yet" line with Measure its limits… | pass. The capture (a7d543b10, 2026-11-01) shows the card before D-62's name and the review's one-line "Not measured yet"; the gallery gate re-shoots it | PASS | 50326bbd5 | 2026-11-07 | Claude (self) |
| R3 a profile row shows its badge, name, apps using it, how many are open, and its menu (amended: no folder path, design review #63) | vitest `screens/accounts/accounts-grouped.test.tsx` "an App row says where its sessions are open, and never its own renewal…" (now also: "1 open now" / "Nothing open"); `account-manage` and `item-menu` tests for Sign in…, Edit launch settings, default, remove, Measure its limits… | each part as stated | 11/11 pass | PASS | 4df63a6de | 2026-11-07 | Claude (self) |
| R4 two profiles on one account share one runway; the same login twice is normal, not a warning | `cd ui && pnpm -s exec vitest run src/screens/accounts/accounts-grouped.test.tsx`: "two profiles on one login share one runway: the freshest measured reading", "two subscriptions over six profiles: …" (one `account-runway` per card); cargo (runs in CI) `juggler-app` `src/features/subscriptions/mod.rs::two_profiles_on_one_login_are_one_account`; `grep -rni "same login" ui/src crates` (no warning in the code) | one runway per account; no warning | pass; `shareAccount` true for w1/w2, the freshest reading (w2) shown; no "same login twice" warning exists (ADR-0008 A-2 retires E3's) | PASS | ac0e41dc0 | 2026-11-07 | Claude (self) |
| Gate | `cd ui && pnpm -s exec vitest run src/screens/accounts/accounts-grouped.test.tsx src/components/account-manage.test.tsx src/components/item-menu.test.tsx` | all pass | 3 files, 44 tests pass. Cargo and Playwright (`session-pages-gallery.spec.ts` "Accounts with …") cited as written: they run in CI / the gallery gate | PASS | 051e71e4b | 2026-11-07 | Claude (self) |

## R3's gap, exactly

- **Folder.** `ui/src/screens/accounts/ProfileRow.tsx` no longer draws a
  folder (the header comment: "Its folder is said on its App's page"), and
  `accounts-grouped.test.tsx` asserts there is none. Either the maintainer
  amends R3 (a D-number recording design review #63), or the row shows the
  folder again, muted and truncated, in the `profile-apps` line, with the
  test expecting `profile-folder` per row.
- **How many are open.** Add to `accounts-grouped.test.tsx`: render
  `ProfilesScreen` with two running sessions on one App and none on
  another, and expect that row's `account-activity` to read "2 open now"
  and the other's "Nothing open".

## What exists now

- **Grouping** (`crates/juggler-app/src/features/subscriptions/`,
  `subscriptions_list`): each App's sign-in probed through its adapter
  (metadata only) and grouped by tool + account id + organisation; Apps not
  signed in last, an unreadable sign-in alone; the key the UI holds is a
  hash, never a vendor id. D-62: the sign-in is stored as its Account,
  with plan, renewal and fee kept there (`terms.rs`).
- **The page** (`ui/src/screens/Profiles.tsx`, `screens/accounts/AccountCard.tsx`,
  `ProfileRow.tsx`): one card per Account, named its own way (its team, else
  its plan), "email · plan", one runway, or "Not measured yet ▸ · Measure
  its limits…" once; App rows beneath with the mark, name, where they're
  open, n open now, Open and the registry's menu (`lib/action-specs-setup.ts`).

## Binary artifacts

None added; cited: `_gallery/93-accounts-profiles/accounts-{one,two,six}-profiles-1280x800-{dark,light}.png`
(captured 2026-11-01, a7d543b10; predate D-62's names and design review #63).

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here
- [x] Paths hashed or generalised where they reveal private layout (repo paths only)
- [x] Transcript excerpts scrubbed of personal content (none)
