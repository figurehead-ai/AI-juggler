<!-- captures: abe127b3ec17 -->
# 94-service-accounts: accounts that run through another service (OpenCode Go, Kimi), a missing key, and adding one

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design fixes.

## 0.3.2

accounts-key-missing shows the 0.3.2 Accounts page well: the "Go and Go Codex" (OpenCode Go · Go Plus · estimate) and Kimi cards stand a step off the dark canvas and as white cards on grey in light, the "Estimate" and "Unknown" pills say up front how far each number can be trusted, and the missing Kimi key is said three ways (the head line "Its Kimi Code API key isn't in this Mac's Keychain", "Add its key…" on the right, and the amber "Its key isn't here" on the profile row). accounts-service is now re-rendered on the 0.3.2 screen: the same two cards with a healthy Kimi ("Kimi · Pro · usage unknown", Replace key…), the 0.3 top bar and rail, and shortened profile folders (~/…/go, ~/…/go-codex, ~/…/kimi) in place of the long Application Support paths, so it now matches its sibling. Add an account is the one solid violet button on both Accounts captures. In add-account-more both Claude Code → and Codex → are solid violet, so the dialog has two equal primaries while the OpenCode Go → and Kimi → rows below keep outline buttons; the installed tools rightly lead, but nothing picks one of the two. service-sheet reads cleanly top to bottom (plan, key with its Keychain sentence, a block per tool with model and profile name, the Estimate note, then Cancel and a solid Add account); the focused Codex model field still shows the placeholder-like value "a-model". The design fixes show in dark as the lighter accent on the logo mark, Add an account, New session, the Estimate pill, the "See it on … site →" links, and in service-sheet the two ticked checkboxes and the Codex field's focus ring; the light captures changed only in the top bar, which stays one row with the whole "7 to clean up" pill. Claude Code and Codex share the same circled-C mark, so on the Go and Go Codex rows and in service-sheet's "Use it with" blocks the mark doesn't tell the two tools apart; only the words do.

## Weak

- add-account-more: Claude Code → and Codex → are both solid violet, two primaries side by side in one dialog.
- service-sheet: the Codex model field holds "a-model", which reads as a placeholder rather than a real model name.
- accounts-service, accounts-key-missing, service-sheet: the Codex rows and block carry the same circled C as Claude Code ("© Go Codex", "© Codex"), so the tool mark doesn't distinguish them.
