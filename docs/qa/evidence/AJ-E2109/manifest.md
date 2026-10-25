# Evidence manifest — AJ-E2109

Task: `docs/tasks/E21-desktop-first.md` AJ-E2109, Accounts are
subscriptions; profiles sit under them (D-35). Branch `e21-pages`,
2026-10-24.
Status: **IN-PROGRESS**: grouping, the page and the menu are built and
tested on fixtures; open are the pinned container's gallery run and a
look on the maintainer's real setup (two subscriptions over six
profiles).

Gallery: `docs/qa/evidence/_gallery/93-accounts-profiles/` (one, two and
six profiles, both themes; local Playwright run, the container run
replaces them).

## What exists now

- **Grouping, worked out when read** (`crates/juggler-app/src/features/subscriptions/`,
  `subscriptions_list`): every stored profile (still one row per folder:
  no database or schema change) is probed for its sign-in through its
  adapter (`probe_identity`: metadata only), and grouped by tool +
  account id + organisation.
  - The probe now carries the organisation (`IdentityProbe::SignedIn.org`:
    Claude Code's `organizationUuid`; Codex's account id is the
    workspace's already). It's compared, never shown; the wrong-account
    guard still compares the subject alone, so no remembered identity
    changes.
  - A personal plan and a team seat on one email are two accounts.
  - Profiles not signed in come last, one group per tool ("Not signed
    in"); a sign-in that can't be read (a Codex API key) stands alone.
  - Plan, renewal and fee (`subscription_fee:<profile>`) are the
    account's when every profile that names one agrees; otherwise
    `differs` names them and each profile row shows its own.
  - The key the UI holds is a hash, never a vendor id; a personal
    organisation (Claude names it after the email) is never shown.
- **The page** (`ui/src/screens/Profiles.tsx`, now ~400 lines, with
  `screens/accounts/AccountCard.tsx`, `ProfileRow.tsx`, `UseOriginal.tsx`):
  - "Accounts", and the line "An **account** is a subscription you pay
    for. A **profile** is a setup an app runs on: a Claude app profile or
    a Claude Code folder. One account can have several profiles." Add an
    account.
  - One card per account: the tool's mark, its name (the team's, else
    "Claude Max"), "email · plan" (and renewal and fee where they agree),
    "n profiles share this account", the 5-hour and week meters (one
    runway, the freshest measured reading among its profiles), or "Not
    measured yet" with "Its sessions run outside AI Juggler, so limits
    aren't measured yet. Measure its limits…" (opens the profile's page,
    where AJ-E1530's opt-in lives).
  - Profile rows beneath: the colour badge with its monogram, name,
    Default, the apps using it now ("Claude app, Terminal", else its
    tool), its folder (mono, muted), "n open now" or "Not signed in" with
    Sign in, Open, and its ⋯ menu.
  - The menu is the registry's (`action-specs-setup.ts`): Open its page,
    Start a session, Sign in…, Edit launch settings, **Measure its
    limits…** (new), Rename…, Copy settings from…, Make the default,
    Remove…. Edit launch settings unfolds the profile's launch settings
    under its row.
- **Elsewhere**: a session's page reads its account from the same
  grouping (the Account chip, Runway, "no new room"); Accounts points at
  Settings › Switching for context packs.
- **"Same login twice"**: no such warning exists in the code (E3's
  requirement was never built), so there was nothing to remove; two
  profiles on one login now read as one account with one runway.
- **Terminology lint**: the JSX rule allows D-35's definition on screen
  ("a setup an app runs on", "several profiles", "profiles share this
  account"), with a comment citing D-35.

## Tests

| Requirement | Test |
|---|---|
| One login on two profiles is one account | `crates/juggler-app/src/features/subscriptions/mod.rs::two_profiles_on_one_login_are_one_account` |
| Personal vs team seat on one email stay apart | `…subscriptions/mod.rs::a_personal_plan_and_a_team_seat_on_one_email_stay_apart`, `crates/juggler-adapters/tests/claude_identity.rs::a_personal_plan_and_a_team_seat_on_one_login_name_their_organisations` |
| One login on two tools is two accounts | `…subscriptions/mod.rs::one_login_on_two_tools_is_two_accounts` |
| Not signed in last; unreadable stands alone | `…subscriptions/mod.rs::not_signed_in_comes_last_and_an_unreadable_sign_in_stands_alone` |
| Plan, renewal, fee when they agree | `…subscriptions/mod.rs::plan_renewal_and_fee_are_the_accounts_when_its_profiles_agree` |
| No vendor id in the key | `…subscriptions/mod.rs::the_key_never_carries_the_vendor_ids` |
| The guard still compares the subject | `crates/juggler-app/src/identity.rs::the_verdict_never_passes_a_different_identity` |
| 1, 2 and 6 profiles; one runway; Not signed in last; per-profile values | `ui/src/screens/accounts/accounts-grouped.test.tsx` |
| The menu | `ui/src/components/account-manage.test.tsx` |
| Both themes, axe 0 | `ui/tests/visual/session-pages-gallery.spec.ts` (6 of its 14) |

Gates: as AJ-E2108's manifest (same branch, same run).

## Not done

- Renewal and fee are still entered per profile (the stored entity stays
  per folder until 0.2); the account shows them where they agree.
- The pinned container's run; a look at the grouping on the
  maintainer's setup.
