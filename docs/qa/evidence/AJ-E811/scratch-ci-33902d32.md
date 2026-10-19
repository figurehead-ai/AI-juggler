<!-- Copied verbatim from the sweep report's summary.md. Of its images, only the recovery-code triptych is committed (scratch-recovery-code-*.png); the rest stayed in the report. -->
## Visual full sweep (D-21)

Base `dd494e6cb91e` (merge-base with main) → head `33902d32a37c` (claude/visual-sweep-ci-proof).
921 states: **6 changed**, 4 unstable, 0 only at head, 0 only at base, 911 unchanged.

| Changed state | Differing pixels | Share | Size | Changed region (x, y, w, h) |
|---|---:|---:|---|---|
| `37-flows/a4-signins-1024x768-light.png` | 2422 | 0.31 % | 1024x768 | 266, 509, 88, 28 |
| `59-vault/confirmed-1280x800-light.png` | 367 | 0.04 % | 1280x800 | 467, 396, 66, 30 |
| `59-vault/recovery-code-1280x800-light.png` | 367 | 0.04 % | 1280x800 | 467, 396, 66, 30 |
| `59-vault/confirmed-1280x800-dark.png` | 361 | 0.04 % | 1280x800 | 467, 396, 66, 30 |
| `59-vault/recovery-code-1280x800-dark.png` | 361 | 0.04 % | 1280x800 | 467, 396, 66, 30 |
| `63-consent/first-run-1280x800-light.png` | 44 | 0.0043 % | 1280x800 | 555, 637, 6, 9 |

Unstable — rendered differently twice on the same commit, so the sweep can't tell whether the change touched them (images in the report):

- `63-consent/first-run-1280x800-dark.png` (head; base→head 52 px)
- `37-flows/a2-proposal-1280x800-light.png` (base; base→head 45 px)
- `34-wizard/scan-progress-1280x800-dark.png` (base; base→head 37 px)
- `36-recommend/proposal-1280x800-light.png` (base and head; base→head 37 px)

Images: `states/<screen>/<state>.{base,head,diff}.png`; open `index.html` to review them side by side.
