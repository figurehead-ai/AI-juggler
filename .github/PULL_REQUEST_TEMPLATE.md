## What and why

<!-- What does this change, and why? Link the issue it addresses ("Closes #123"). -->

## Checklist

- [ ] `just verify` passes
- [ ] Tests cover the change (new behaviour, or the bug this fixes)
- [ ] Docs and the CHANGELOG are updated if users will notice the change
- [ ] **Nothing personal or secret**: no real account names, emails, home paths, hostnames, tokens, or screenshots of a real desktop
- [ ] Every commit is signed off ([DCO](../CONTRIBUTING.md#licensing-and-sign-off), `git commit -s`)

<details>
<summary>For larger or interface changes (the full <a href="../docs/plan/QUALITY.md">Definition of Done</a>)</summary>

- [ ] Typed errors; no `unwrap`, `expect` or `panic!` in production code; public APIs documented
- [ ] Layering respected, nothing tool-specific outside an adapter, no new duplication ([engineering guide](../docs/engineering/ENGINEERING-GUIDE.md))
- [ ] No new `unsafe`; `cargo deny check` clean; any new attack surface has its threat-model row and tests ([security guidelines](../docs/engineering/SECURITY-GUIDELINES.md))
- [ ] Interface changes: both themes checked, `pnpm --dir ui exec playwright test` and `just visual-gate` pass, and an accessibility scan covers the new state
- [ ] Design quality (D-57): each new or changed screen's gallery family has its critique (`docs/qa/evidence/_gallery/<family>/critique.md`) against the checklist and the reference screens (the row menus, the rail's order menu, Usage, Processes): one grid, a clear hierarchy, consistent spacing, icons on menu and action lines, the shared controls, disabled states that say why, plain words, nothing under 12 px, both themes at 1280×800 and 1024×700; every new popover, menu and sheet captured; what it found is fixed; `python3 scripts/critique_check.py` passes
- [ ] An ADR when this makes an architecture decision

</details>

## Visual baselines

<!-- Only when this PR changes files under ui/tests/visual/__baselines__/:
     name the issue (#123) or task (AJ-E…) and, one line per changed baseline,
     its file name and the intentional change, for example
     `settings-dark-comfortable-1280x800: Privacy card added`.
     CI rejects baseline changes without a note for each. -->

## Notes for reviewers

<!-- Anything non-obvious, trade-offs, or where you'd like a second opinion. -->
