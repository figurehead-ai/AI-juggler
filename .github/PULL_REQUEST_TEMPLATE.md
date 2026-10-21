## What & why

<!-- What does this PR change, and why? Link the issue it addresses ("Closes #123"). -->

## Definition of Done ([docs/plan/QUALITY.md](../docs/plan/QUALITY.md) — required for every PR)

- [ ] Acceptance criteria from the issue demonstrably pass
- [ ] `just verify` clean (fmt, clippy `-D warnings`, tests, docs) — zero warnings
- [ ] Public APIs documented; typed errors; no `unwrap`/`expect`/`panic!` in production paths
- [ ] Tests added (unit + integration/fixtures for new flows; corpus grown if parsers touched)
- [ ] Architecture layering respected (QUALITY.md §4); no new `unsafe`; `cargo-deny` clean
- [ ] Engineering guide checklist passes (docs/engineering/ENGINEERING-GUIDE.md): right module, no tool-id literal outside adapters, thin async commands, typed errors, no new duplication
- [ ] Security guidelines followed (docs/engineering/SECURITY-GUIDELINES.md); any new attack surface has its threat-model row and tests
- [ ] CHANGELOG updated; spec/docs updated if behavior changed; ADR if a decision was made
- [ ] UI changes: both themes captured in the gallery, a11y scan added, `pnpm --dir ui exec playwright test` green, `just visual-gate` green (or baselines re-rendered with a note per screen)
- [ ] Evidence stored under `docs/qa/evidence/<task-id>/` when this closes a task
- [ ] **Nothing personal or secret**: no real account names, e-mails, home paths, hostnames, tokens, or screenshots of a real desktop
- [ ] Every commit is signed off ([DCO](../CONTRIBUTING.md#licensing-and-sign-off), `git commit -s`)

## Visual baselines

<!-- Only when this PR changes files under ui/tests/visual/__baselines__/ (AJ-E803):
     name the task (AJ-E…) and, one line per changed baseline, its file name and the
     intentional change — e.g. `settings-dark-comfortable-1280x800: Privacy card added`.
     CI rejects baseline changes without a note for each. Maintainer approval required. -->

## Visual baselines

<!-- Only when this PR changes files under ui/tests/visual/__baselines__/ (AJ-E803):
     name the task (AJ-E…) and, one line per changed baseline, its file name and the
     intentional change — e.g. `settings-dark-comfortable-1280x800: Privacy card added`.
     CI rejects baseline changes without a note for each. Maintainer approval required. -->

## Reviewer checklist

Structure & naming · layering · error handling · test quality · docs · CHANGELOG · no secrets · bench impact on hot paths

## Notes for reviewers

<!-- Anything non-obvious, tradeoffs made, or things you want a second opinion on. -->
