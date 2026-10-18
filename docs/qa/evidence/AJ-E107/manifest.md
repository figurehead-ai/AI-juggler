# Evidence manifest — AJ-E107 (bridge + generated-types rename)

| Claim | Command | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|---|
| Req.1 DTO renames + serde alias | cargo test --workspace | green; `harness` deserialises one version | ProviderInfo/LaunchRecipeInfo/InstanceInfo with `#[serde(alias="harness")]`; 84 tests | PASS | 2026-10-13 | zcode-primary |
| Req.2 canonical command names + legacy aliases | handler registration (`commands::legacy_aliases::*`) | both callable, payloads unchanged | providers_list/launch_recipes_list/instances_list canonical; legacy names registered | PASS | 2026-10-13 | zcode-primary |
| Req.3 TS regenerated + deprecated re-exports | ts-rs export test + generated dir | new files; old re-export | ProviderInfo/InstanceInfo/LaunchRecipeInfo.ts generated; Harness/Profile/SessionInfo.ts re-export as deprecated | PASS | 2026-10-13 | zcode-primary |
| Req.4 bridge canonical + aliases; call sites canonical | tsc --noEmit + grep | 0 non-bridge legacy uses outside deprecated markers | launchRecipesList/instancesList/providersList canonical; @deprecated aliases; UI on canonical | PASS | 2026-10-13 | zcode-primary |
| End-to-end preview | pnpm visual:capture | green | 41 captures green with canonical names + new wire fields | PASS | 2026-10-13 | zcode-primary |
