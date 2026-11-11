# Evidence manifest — AJ-E2009

Task: `docs/tasks/E20-engineering-and-security.md` AJ-E2009, incident
readiness. Branch `release/0.4.1` (D-83). Status: **DONE-EVIDENCED** (agent
self-check, 2026-11-06).

| Req | Result | Evidence |
|---|---|---|
| R1 | PASS | `docs/RELEASING.md`, "If a key leaks", covers four things. **Updater key rotation:** a runtime list of trusted public keys. `updates::trusted_keys` gives the current `AIJ_UPDATER_PUBKEY`, then the standby `AIJ_UPDATER_NEXT_PUBKEYS`, both baked in by `build-macos.yml`. `PluginFeed` retries a package that fails the current key with each standby key. `tests/updates_plugin.rs`: `a_release_signed_with_a_standby_key_installs_and_an_unknown_key_never_does` and `the_trusted_keys_are_the_current_one_then_the_standby_ones` pass, with the real updater plugin against a local feed. **Developer ID revocation and replacement.** **A secret rotation calendar**, with what is due when. **Incident communications:** an advisory within 24 hours, the fixed release's notes, the website notice, and the reporter. |
| R2 | PASS | `SECURITY.md` covers four things. **Channels and versions:** Stable (the first is a 0.4.x, D-85), Beta and Nightly, with older releases unsupported. **Safe harbour** for good-faith research. **Disclosure:** a 90-day target. **CVEs:** through GitHub security advisories. It also gives a fallback email, `info@figurehead.ai`: the address the website's `security.txt` already publishes. It links the runbook. |
| R3 | PASS | `https://figurehead.ai/.well-known/security.txt` is live (fetched 2026-11-06). It has Contact (`info@figurehead.ai` and the GitHub advisory form), Expires 2027-10-16, Preferred-Languages, Canonical and Policy. It is in the website repo (`public/.well-known/security.txt`). |

One step is the maintainer's, once: generate the standby updater key
offline and set the repository variable `AIJ_UPDATER_NEXT_PUBKEYS`
(RELEASING.md, "One-time setup"). Until then, builds trust only the
current key, as before.
