<!-- Copied verbatim from the sweep report's summary.md. Of its images, only the recovery-code triptych is committed (scratch-recovery-code-*.png); the rest stayed in the report. -->
## Visual full sweep (D-21)

Base `dd494e6cb91e` (origin/main) → head `bd82c7aca06f` (HEAD).
921 states: **5 changed**, 3 unstable, 0 only at head, 0 only at base, 913 unchanged.

| Changed state | Differing pixels | Share | Size | Changed region (x, y, w, h) |
|---|---:|---:|---|---|
| `59-vault/confirmed-1280x800-light.png` | 367 | 0.04 % | 1280x800 | 467, 396, 66, 30 |
| `59-vault/recovery-code-1280x800-light.png` | 367 | 0.04 % | 1280x800 | 467, 396, 66, 30 |
| `59-vault/confirmed-1280x800-dark.png` | 361 | 0.04 % | 1280x800 | 467, 396, 66, 30 |
| `59-vault/recovery-code-1280x800-dark.png` | 361 | 0.04 % | 1280x800 | 467, 396, 66, 30 |
| `37-flows/a2-proposal-1280x800-dark.png` | 37 | 0.0036 % | 1280x800 | 555, 494, 6, 9 |

Unstable — rendered differently twice on the same commit, so the sweep can't tell whether the change touched them (images in the report):

- `63-consent/first-run-1280x800-light.png` (head; base→head 53 px)
- `63-consent/first-run-1280x800-dark.png` (head; base→head 51 px)
- `36-recommend/proposal-1280x800-dark.png` (head; base→head 37 px)

Images: `states/<screen>/<state>.{base,head,diff}.png`; open `index.html` to review them side by side.
