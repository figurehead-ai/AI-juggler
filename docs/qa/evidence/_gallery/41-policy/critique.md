<!-- captures: 5c587160818f -->
# 41-policy: a profile's launch settings, its keys and passwords choice, and deleting launch settings

Design critique (D-57, AJ-E2814) against the reference screens: the row menus (77-menus), Settings' cards (98-local-cache) and the remove-account sheet. The baseline note says what changed on purpose; this says whether it is good. First critique of this family: its captures were last taken at 0.1 and were re-rendered for 0.3.1 (D-60).

## Checklist

| Check | Finding |
|---|---|
| Grid | Launch settings sit in one card under their profile: name, model chip, Options, then delete and Launch on the right edge. Settings › Switching is one card of three choices, then the per-tool pickers. |
| Hierarchy | Account, then profile, then "Launch settings · used when you start on this profile"; in Switching, "Your choice (all tools)" over its three lines, then "Per-tool choice". |
| Spacing | The settings card's rows keep the account card's rhythm; the env override row lines its fields up with the name above. |
| Icons | Delete is the bin; Launch carries the terminal mark; the confirm has no icon, as the remove-account sheet. |
| Shared controls | The shared Picker for "If a session crashes" and the per-tool choice (E28); radios with a hint under each label; the shared Modal for the confirm. |
| Disabled reasons | "Save overrides" is greyed until a value changes. |
| Plain words | "Hide them (recommended)", "Leave them in", "Ask on every switch"; the confirm: "It's removed from this account's launch settings and won't start new sessions. Sessions already made with it stay in History." |
| Type sizes | Names 14 px, hints and labels 12–13 px. |
| Both themes | Checked dark and light. |
| 1280×800 and 1024×680 | Both; the folded rail at 1024 leaves the settings their width. |

## Good

- Deleting launch settings asks first and says the one thing a user worries about: the sessions made with them stay. A refusal (an open session) is shown in the backend's own words, naming the session.
- The keys and passwords choice says what each option means for the user, with the safe one marked as recommended.

## Weak

- The per-tool choice lists the preview's stand-in tool by its id ("fake-desktop"). Real tools show their names (`providerDisplayName`); only the preview's fixture lacks one.
- The confirm says nothing about the profile's other launch settings, so deleting the last one leaves the profile with none and its Start disabled ("Add launch settings first"). That is said on the profile afterwards, not before.
- accounts-pack-secrets-*: four solid violet buttons in one view at 1280 (New session, Add an account, and Launch on Deep work and on Quick fixes; three at 1024), so none of them reads as the page's primary.
- accounts-pack-secrets-*: the disabled "Save overrides" under the env override is a pale violet that reads as pressed, not unavailable.
- accounts-pack-secrets-1280x800-*: the rail rows read "Working · 748 d" and "Ended · 748 d ago"; the fixture's clock makes the ages look broken.

## Fixed

- 0.3.1 (D-60): one click on the bin deleted launch settings and, through the sessions table's cascade, every session made with them. It now asks first; the settings are marked removed, start nothing, and their sessions stay in History; while one is open the delete is refused, naming it. Vitest: `delete-launch-settings.test.tsx`; Rust: `manage_accounts.rs` "deleting_launch_settings_keeps_their_sessions_and_waits_for_open_ones".

## Captures

| Capture | What is on it |
|---|---|
| accounts-pack-secrets-1280x800, -1024x680 | Work Max's profile with its launch settings open: Deep work (opus) with an override CLAUDE_CODE_MAX_OUTPUT_TOKENS = 8000 and "If a session crashes: As in Settings", Quick fixes (sonnet). |
| settings-pack-secrets-1280x800, -1024x680 | Settings › Switching › Handovers and keys: Hide them (recommended), Leave them in, Ask on every switch; not chosen yet; the per-tool choice for each tool. |
| accounts-delete-launch-settings-1280x800, -1024x680 | "Delete Deep work?" over the Accounts page: it's removed from the launch settings and starts nothing new; its sessions stay in History; Cancel · Delete. |

## 0.3.2

The dark Accounts page (accounts-pack-secrets) separates its layers: the Work Max account card stands off the canvas and the Launch settings box sits a step lighter inside it, and in light the white card has a soft edge on the grey canvas. The lighter dark accent shows on New session, Add an account and both Launch buttons, which makes the remaining problem plainer: four solid violet buttons in one view at 1280, so there is no single primary and Launch competes with Add an account. The disabled "Save overrides" is still a pale violet that reads like a pressed button rather than an unavailable one. Settings at 1024 (settings-pack-secrets-1024x680) now has its "Switching" page title above "Handovers and keys", as the 1280 capture already did, so the first pass's missing title is fixed; the three option boxes under "Your choice (all tools)" are a step apart from their card, the hints under each label are readable, and the per-tool pickers still list the preview tool as "fake-desktop". The rail at 1280 now shows "Stopped with an error" whole, without the cut-off age, but the fixture's other rows read "Working · 748 d" and "Ended · 748 d ago", an age no real session has. The delete confirm keeps its plain wording with a red-tinted Delete. All captures were re-rendered.
