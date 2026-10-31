<!-- captures: 3f794350b8a3 -->
# 94-service-accounts: accounts that run through another service (OpenCode Go, Kimi), a missing key, and adding one

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now.

## 0.3.2

accounts-key-missing shows the 0.3.2 Accounts page well: the OpenCode Go and Kimi cards stand a step off the dark canvas and as white cards on grey in light, the "Estimate" and "Unknown" pills say up front how far each number can be trusted, and the missing Kimi key is said three ways (the head line "Its Kimi Code API key isn't in this Mac's Keychain", "Add its key…" on the right, and the amber "Its key isn't here" on the profile row). In add-account-more both Claude Code → and Codex → are now solid violet, so the dialog has two equal primaries while the OpenCode Go → and Kimi → rows below keep outline buttons; the installed tools rightly lead, but nothing picks one of the two. service-sheet reads cleanly top to bottom (plan, key with its Keychain sentence, a block per tool with model and profile name, the Estimate note, then Cancel and a solid Add account); the focused Codex model field shows the placeholder-like value "a-model". accounts-service-1280x800 (dark and light) was not re-rendered: it still shows the old top bar (Home · Accounts · History · Settings · More) and full ~/Library/Application Support/… profile paths, so its colours and layout say nothing about 0.3.2.

## Weak

- add-account-more: Claude Code → and Codex → are both solid violet, two primaries side by side in one dialog.
- service-sheet: the Codex model field holds "a-model", which reads as a placeholder rather than a real model name.
- accounts-service: not re-rendered; it shows the retired top bar and long unshortened profile paths, unlike its sibling accounts-key-missing.
