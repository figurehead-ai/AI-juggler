# AJ-E910 check outputs (2026-10-13)

## grep -n "Provider\|Instance\|SwitchEdge" docs/SPEC.md (head)
88:Vocabulary (user-facing and normative in this spec): a **Provider** is a
90:OpenCode…); an **Account** is one subscription/login under a Provider; an
91:**Instance** is one running occurrence of a tool under one Account on one
92:project; a **Session** is the conversation inside an Instance. "Profile"
101:pub type ProviderId = HarnessId;         // ADR-0008 alias (additive)
105:    pub harness: HarnessId,              // UI: "Provider"
118:pub struct SessionRecord {               // domain alias target: InstanceRecord
120:    pub kind: InstanceKind,              // Terminal | Desktop (desktop = windowed
148:  every switch; Instances come and go underneath it.
149:- **Node (instance-epoch)** — one Instance running under one Account from
154:- **Edge (`SwitchEdge`)** — one executed switch: `id, work_id,
195:is the **user's remembered choice** (global policy + per-Provider

## lossless hits + justification
docs/SPEC.md:420:the switch itself. The word *lossless* is reserved for same-Provider
docs/SPEC.md:480:      `lossless` restricted to proven same-Provider cases
README.md:49:beta with per-pair fidelity badges — never marketed as lossless;
docs/PROPOSAL.md:99:- **Same tool, same account:** native resume — lossless.
docs/plan/EPICS.md:146:| M4-1 | Transcript IR completion | All three parsers → neutral IR; golden-file tests per harness × version | IR round-trips (lossless where format allows); golden diffs fail loudly on format drift |

## fleet deferral language
docs/SPEC.md:18:| Platform (v0.x) | macOS 13+ (Universal: Apple Silicon + Intel). Windows/Linux post-1.0; the Rust core and adapters are cross-platform from day one (CI builds all three OSes) so ports need no rewrites |
docs/SPEC.md:689:7. **Platform: macOS-first (Universal, v0.x); Windows/Linux post-1.0; cross-platform core from day one.** ✅

## baseline gates
just verify: GREEN (29 suites, pre-change baseline)
pnpm --dir ui build: GREEN
git diff --stat: docs/*, docs/qa/*, README.md only (see PR)
