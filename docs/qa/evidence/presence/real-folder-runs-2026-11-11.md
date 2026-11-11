# Terminal runs on the maintainer's real profile folders (read-only listing, 2026-11-11)

Every Claude Code session file in `ClaudeCodeProfiles/{bookauth-*,personal-max}` whose first `entrypoint` is `cli` or `sdk-cli`: started time (UTC), folder, entrypoint, where it ran, file id prefix. All were agents' labs: the processes, tasks, outside, messaging and runtime labs (10-25 to 10-28, from AI Juggler Dev) and the switch and cut labs (`examples/switch_lab.rs`, 11-02 to 11-07). None is deleted; the guards in ADR-0032's evidence stop new ones.

| Started | Folder | Entrypoint | Ran in | File |
|---|---|---|---|---|
| 2026-10-25T17:23 | bookauth-luka | sdk-cli | `~/Dev/aij-lab-0.2-2026-10-25` | 3782bfee |
| 2026-10-25T17:25 | bookauth-luka | sdk-cli | `~/Dev/aij-lab-0.2-2026-10-25` | 27a9f8bf |
| 2026-10-25T21:18 | bookauth-luka | cli | `~/Dev/aij-lab-runtime-2026-10-25` | 4432a96a |
| 2026-10-25T21:24 | bookauth-luka | sdk-cli | `~/Dev/aij-lab-runtime-2026-10-25` | b18540ef |
| 2026-10-26T19:49 | bookauth-luka | cli | `~/lab/aj-messaging` | a1c6b486 |
| 2026-10-26T20:04 | bookauth-luka | cli | `~/lab/aj-tasks` | f94be0f3 |
| 2026-10-26T20:13 | bookauth-luka | cli | `~/lab/aj-tasks` | c9f0c252 |
| 2026-10-26T20:19 | bookauth-luka | cli | `~/lab/aj-processes` | 4e50ed02 |
| 2026-10-26T20:21 | bookauth-luka | cli | `~/lab/aj-processes` | 02746f6e |
| 2026-10-26T20:22 | bookauth-luka | cli | `~/lab/aj-processes` | 3a8c43ab |
| 2026-10-26T20:25 | bookauth-luka | cli | `~/lab/aj-processes` | 3db28085 |
| 2026-10-26T20:27 | bookauth-luka | cli | `~/lab/aj-processes` | 3e9852ab |
| 2026-10-26T20:29 | bookauth-luka | cli | `~/lab/aj-processes` | ee2444d7 |
| 2026-10-26T20:32 | bookauth-luka | cli | `~/lab/aj-processes` | 44695785 |
| 2026-10-26T20:34 | bookauth-luka | cli | `~/lab/aj-processes` | 6a85416a |
| 2026-10-26T20:37 | bookauth-luka | cli | `~/lab/aj-processes` | ba37c507 |
| 2026-10-26T20:38 | bookauth-luka | cli | `~/lab/aj-processes` | c69dd31e |
| 2026-10-26T20:40 | bookauth-luka | cli | `~/lab/aj-processes` | d371996a |
| 2026-10-26T20:47 | bookauth-luka | cli | `~/lab/aj-processes` | dfd949c0 |
| 2026-10-26T20:49 | bookauth-luka | cli | `~/lab/aj-processes` | ad327d5a |
| 2026-10-28T16:58 | bookauth-luka | cli | `~/lab/aj-messaging-clean` | e64d3a51 |
| 2026-10-28T17:05 | bookauth-luka | cli | `~/lab/aj-messaging-clean2` | 6487bcf6 |
| 2026-10-28T17:12 | bookauth-luka | cli | `~/lab/aj-tasks-clean` | b2b6aae3 |
| 2026-10-28T17:15 | bookauth-luka | cli | `~/lab/aj-tasks-clean` | 147d4349 |
| 2026-10-28T17:25 | bookauth-luka | cli | `~/lab/aj-tasks-clean2` | 4d8edd03 |
| 2026-10-28T17:29 | bookauth-luka | cli | `~/lab/aj-tasks-clean2` | 39720593 |
| 2026-10-28T17:37 | bookauth-luka | cli | `~/lab/aj-tasks-clean3` | eb18d81e |
| 2026-10-28T17:40 | bookauth-luka | cli | `~/lab/aj-tasks-clean3` | 216f6127 |
| 2026-10-28T17:48 | bookauth-luka | cli | `~/lab/aj-tasks-clean4` | 29e47a7c |
| 2026-10-28T17:51 | bookauth-luka | cli | `~/lab/aj-tasks-clean4` | 19054ac1 |
| 2026-10-28T18:01 | bookauth-anela | cli | `~/lab/aj-tasks-clean5` | 1cc7da59 |
| 2026-10-28T18:01 | bookauth-luka | cli | `~/lab/aj-tasks-clean5` | 1cc7da59 |
| 2026-10-28T18:04 | bookauth-luka | cli | `~/lab/aj-tasks-clean5` | afb3a3af |
| 2026-10-28T18:13 | bookauth-anela | sdk-cli | `~/lab/aj-switch-clean` | c1ea2430 |
| 2026-10-28T18:13 | bookauth-luka | sdk-cli | `~/lab/aj-switch-clean` | c1ea2430 |
| 2026-10-28T18:17 | bookauth-anela | sdk-cli | `~/lab/aj-switch-clean` | ba076d05 |
| 2026-10-28T18:22 | bookauth-anela | sdk-cli | `~/lab/aj-switch-clean` | 0dea93f1 |
| 2026-10-28T18:24 | bookauth-anela | sdk-cli | `~/lab/aj-switch-clean/.wt/egret` | 274ff354 |
| 2026-10-28T18:24 | bookauth-luka | sdk-cli | `~/lab/aj-switch-clean/.wt/egret` | 274ff354 |
| 2026-10-28T18:29 | bookauth-luka | sdk-cli | `~/lab/aj-outside-clean/j1` | 9a643b8c |
| 2026-10-28T18:39 | bookauth-luka | sdk-cli | `~/lab/aj-outside-clean/j2/.wt/tern` | 6a99d4e4 |
| 2026-10-28T18:40 | bookauth-luka | sdk-cli | `~/lab/aj-outside-clean/j2/.wt/tern` | 8ae480c8 |
| 2026-10-28T18:40 | bookauth-luka | sdk-cli | `~/lab/aj-outside-clean/j2/.wt/tern` | ef971d66 |
| 2026-10-28T18:42 | bookauth-luka | sdk-cli | `~/lab/aj-outside-clean/j3/.wt/wren` | 332939ec |
| 2026-10-28T18:46 | bookauth-anela | cli | `~/lab/aj-tasks-clean6` | f06aa3d6 |
| 2026-10-28T18:46 | bookauth-luka | cli | `~/lab/aj-tasks-clean6` | f06aa3d6 |
| 2026-10-28T18:49 | bookauth-luka | cli | `~/lab/aj-tasks-clean6` | 1d9f5fa2 |
| 2026-10-28T20:24 | bookauth-anela | cli | `~/lab/aj-tasks-final` | 1955be25 |
| 2026-10-28T20:24 | bookauth-luka | cli | `~/lab/aj-tasks-final` | 1955be25 |
| 2026-10-28T20:27 | bookauth-luka | cli | `~/lab/aj-tasks-final` | ea86b036 |
| 2026-10-28T21:57 | bookauth-anela | sdk-cli | `~/lab/notes-175453` | 02709f43 |
| 2026-10-28T21:57 | bookauth-luka | sdk-cli | `~/lab/notes-175453` | 02709f43 |
| 2026-10-28T21:58 | bookauth-luka | sdk-cli | `~/lab/notes-175453` | b9b034ad |
| 2026-11-02T14:16 | bookauth-anela | sdk-cli | `~/Dev/aij-switch-lab` | a71cc8f5 |
| 2026-11-02T14:16 | bookauth-luka | sdk-cli | `~/Dev/aij-switch-lab` | a71cc8f5 |
| 2026-11-02T14:42 | bookauth-luka | sdk-cli | `~/Dev/aij-switch-lab` | 4e6b1b3e |
| 2026-11-02T14:42 | bookauth-luka | sdk-cli | `~/Dev/aij-switch-lab` | fcfe474c |
| 2026-11-05T00:59 | bookauth-luka | sdk-cli | `~/lab/cut-20261104-195927` | be3f498c |
| 2026-11-05T00:59 | bookauth-luka | sdk-cli | `~/lab/cut-20261104-195927` | e95b1348 |
| 2026-11-05T05:02 | bookauth-luka | sdk-cli | `~/lab/cut-20261105-000206` | 7ae04ee6 |
| 2026-11-07T13:26 | bookauth-luka | sdk-cli | `~/Dev/aij-switch-lab-e808-0825` | 00d5de81 |
| 2026-11-07T13:33 | bookauth-luka | sdk-cli | `~/Dev/aij-switch-lab-e808-0825/.wt/egret` | 40f49105 |
