# Evidence manifest — AJ-E1558

Task: `docs/tasks/E15-spec-gaps.md` AJ-E1558, Settings, the edit and
create sheets and the pickers in rich mode and colour (D-126, PAT-COLOR),
audited, fixed and enforced (P0). Found by the maintainer on the 0.5.1
Nightly (2026-11-25): "monochrome hell again" on the Settings, edit and
selection pages. With it, the decided amendment to AJ-E1557: the Needs
you rows lose their leading App dot.
Status: **DONE (0.5.2)**, 2026-11-25.

Commits: 4b76afc5c (the Needs you rows lose their App dot), 52c2cc313 and
0d9c22db3 and 80e4da478 (the fixes), a2f4e7e8d (the enforcement: the lint,
the checklist's line 19, critique_check, PAT-COLOR-8), 140f34708 (the
audit), b9c92e236 (the debt baseline lowered), and the baselines, gallery
and evidence commits listed below.

## Requirements

| R | Claim | Verdict | Proof |
|---|---|---|---|
| R1 | An Opus audit of every Settings section, edit/create sheet and picker against DESIGN-SYSTEM (rich mode, icons, PAT-COLOR soft semantic steps, tags/chips, a clear selected state), findings listed | PASS | [`docs/qa/0.5.2-rich-mode-audit.md`](../../0.5.2-rich-mode-audit.md) (140f34708): every pane, the App and Account Settings tabs, every `Modal` sheet and the `DialogFrame` sheets, every picker, read in code and in the captures of 42aab7661 in both themes; 35 findings, each with its surface, what's wrong, the rule and the fix; the usability answers for the changed patterns first; section M, Settings › AI › Models' six findings for AJ-E3826 (the Bot agent's) |
| R2 | Every finding fixed in 0.5.2 | PASS | F1–F35 each name the commit that fixed it (4b76afc5c, 52c2cc313, 0d9c22db3, 80e4da478). Shared pieces in the kit: `RowChip`, `RowLead`, `PickRow` (`controls/ControlRow.tsx`), `SwitchBox` and `.aj-switch` (a native checkbox with `role="switch"`), `ChoiceOption`'s card chip and chosen fill, `MenuItem`'s chosen fill, `Modal`'s `icon`, the new icons (`icons-settings.tsx`, one meaning each); `SettingRow`, `SwitchRow` and `SettingSwitch` require their icon, so the compiler lists any row without one. Tests: vitest (634 files, 4299 tests) pass; `tsc -b` clean on these files |
| R3 | Enforcement: the critique checklist gets a hard "rich-mode colour" line for create/edit/settings, scored, a 3 blocks; a lint or test flags a settings/edit surface built only from neutral tokens. Both fail on the old code | PASS | VALIDATION §5 line 19 and PAT-COLOR-8 (a2f4e7e8d). `scripts/critique_check.py` (in `just evidence`, verify-static) fails a family whose latest critique scores line 19 under 4, and a rich family (`RICH`) whose 0.5.2-or-later section doesn't score it; its selftest proves a 3 fails, a 4 passes, an older section isn't asked. `scripts/rich_mode_lint.py` (in `just terminology`, verify-static) with its only-shrinking allowlist; selftest OK. **On the old code:** the lint run on 42aab7661's `ui/src` (`--root`) exits 1: 40 neutral components in 29 files (28 failing files beyond the allowlist); the line 19 scores the old screens 2 in each family's 0.5.2 critique ("before"), which `critique_check` fails as a latest score |
| R4 | An Opus side-by-side critique in both themes before any baseline | PASS | Each touched family's `critique.md` has a 0.5.2 (AJ-E1558) section: old and new side by side, dark and light, what changed per capture, line 19 scored with the old screens' score beside it; stamped (`critique_check.py --stamp`) |
| R5 | The Needs you rows lose their leading App dot; the need's state mark stays; the App is named in the place line (amendment to AJ-E1557) | PASS | 4b76afc5c: `NeedRow` leads with nothing (the gutter kept), the level's mark before why, the App on the place line (compact: after why). `rail-calm-lenses.test.tsx`, `layout-shift.spec.ts`; AJ-E1557's manifest amended |

## Usability answers, checked on the built screens

1. Someone setting things up finds the section by its tile's icon and the option by its row's chip; what's on is its switch in the accent and its chip lit. Match (65-settings-sections, 84-notifications).
2. Every section and option carries its icon; on, chosen and recommended take the accent's soft step; a warning keeps amber words on the neutral tone; tags say state ("Recommended", "Override", "Can copy"); the chosen card, line or menu value is a fill. Match.
3. The neutral rows are replaced in place by the d126 kit's pieces; no new surface. Match.
4. A one-row section (Your accounts, Desktop apps' empty list) still has its chip and tone. Match.
5. No system prompt: looks only. Match.
6. No colour without meaning (the tiles one hue, navigation's), no bright border added (outlines removed), the kit's AA tokens, no chosen state shown by a ring or a dot alone. Match.

## Checks run

- vitest: 634 files, 4299 tests pass; `tsc -b --noEmit` clean on these files (the only errors are the Bot agent's in-progress `short_name` files); `ui_lint.py`, `design_tokens_lint.py`, `rich_mode_lint.py` (+selftest), `critique_check.py` (+selftest), `plain_words_lint.py`, `native_controls_lint.py`, `platform_lint.py` (+selftest), `debt_baseline.py`: OK.
- `visual_gate.sh` (pinned container): `--update` on a clean export, 258 baselines taken (248 + the 10 the last fixes touched), each diffed and looked at old beside new; the compare run on HEAD then failed only those 10. Notes: [rerender-0.5.2.md](../AJ-E803/rerender-0.5.2.md).
- Gallery: 14 families re-captured on the host (208 changed captures), each family's 0.5.2 critique written and stamped.

## Left

Nothing in scope. Section M of the audit is AJ-E3826's (the Bot agent's): `ModelsJobs.tsx`'s line in `scripts/rich_mode.allow` leaves with its M1 fix. Destructive confirms and decision dialogs drawn outside `Modal` (kill, recovery code, isolation offer) are logged in the audit for the next batch.
