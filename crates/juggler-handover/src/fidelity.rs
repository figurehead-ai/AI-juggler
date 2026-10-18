//! Per-pair fidelity matrix and the capped scoring engine (AJ-E203;
//! pass2/02 §3, deep/09 §H6).
//!
//! Scores are computed per ORDERED pair (`Claude→ZCode` ≠
//! `ZCode→Claude` — source readability vs target primability
//! differ); direction-blind aggregation does not exist here. The
//! composite is a weighted mean, then CAPPED: the cap, not the mean,
//! is the honesty mechanism — no pair advertises fidelity it has not
//! proven. The badge always ships with the carry-list, never alone;
//! T1–T4 tier codes stay engine-internal (deep/09 D2).

use std::collections::BTreeMap;

/// One transferable artifact class (pass2/02 §3.1). The weighted
/// composite uses these eight; summary/compaction enters as the
/// compact CAP, and credentials never transfer at all.
#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, serde::Serialize, serde::Deserialize,
)]
pub enum ArtifactKind {
    /// The chat transcript.
    Chat,
    /// Rules & guidelines (CLAUDE.md / AGENTS.md ring).
    Rules,
    /// Git state & uncommitted diff.
    GitDiff,
    /// In-session todos / plan.
    Todos,
    /// MCP configuration (live processes never transfer).
    Mcp,
    /// Skills / commands / agents.
    Skills,
    /// Terminal env & settings overlay (model names by intent).
    EnvModel,
    /// Subagent state (inline sidechains vs sibling files).
    Subagents,
}

impl ArtifactKind {
    /// The carry-list name (the vocabulary the door's carry summary
    /// renders — AJ-E206; one name per kind, never "context").
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Chat => "chat transcript",
            Self::Rules => "rules & guidelines",
            Self::GitDiff => "uncommitted git diff",
            Self::Todos => "todo list",
            Self::Mcp => "MCP config",
            Self::Skills => "skills & commands",
            Self::EnvModel => "model settings",
            Self::Subagents => "subagent state",
        }
    }
}

/// The artifact weights (pass2/02 §3.4; constants, not tunables).
pub const WEIGHTS: &[(ArtifactKind, f64)] = &[
    (ArtifactKind::Chat, 0.35),
    (ArtifactKind::Rules, 0.15),
    (ArtifactKind::GitDiff, 0.15),
    (ArtifactKind::Todos, 0.10),
    (ArtifactKind::Mcp, 0.10),
    (ArtifactKind::Skills, 0.05),
    (ArtifactKind::EnvModel, 0.05),
    (ArtifactKind::Subagents, 0.05),
];

/// Per-artifact disposition (pass2/02 §3): Copy = 1.0, Translate =
/// 0.8 (0.6 when any key/field is listed-dropped), Regenerate =
/// 0.5, Drop = 0.
#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub enum Disposition {
    /// Byte/move of the native artifact.
    Copy,
    /// Format conversion WITH a written mapping. "Translate" without
    /// a written adapter mapping is Drop (Req.5) — the engine scores
    /// it that way when the caller passes `untranslated: true`.
    Translate {
        /// Any key/field listed as dropped in the conversion.
        keys_dropped: bool,
        /// No written mapping exists — scores as Drop, named in the
        /// carry-list.
        untranslated: bool,
    },
    /// The target rebuilds it (replay turn, re-derived summary).
    Regenerate,
    /// Does not carry; must appear on the carry-list.
    Drop,
}

impl Disposition {
    /// The numeric score this disposition contributes.
    #[must_use]
    pub fn score(self) -> f64 {
        match self {
            Self::Copy => 1.0,
            Self::Translate {
                keys_dropped: false,
                untranslated: false,
            } => 0.8,
            Self::Translate {
                keys_dropped: true,
                untranslated: false,
            } => 0.6,
            Self::Translate {
                untranslated: true, ..
            }
            | Self::Drop => 0.0,
            Self::Regenerate => 0.5,
        }
    }

    /// Carry-list word for the disposition (✓/⚠/✗ render above it).
    #[must_use]
    pub fn carry_word(self) -> &'static str {
        match self {
            Self::Copy => "carried",
            Self::Translate {
                untranslated: true, ..
            } => "stays (no written mapping)",
            Self::Translate { .. } => "translated",
            Self::Regenerate => "rebuilt",
            Self::Drop => "stays",
        }
    }
}

/// The F-label set (deep/09 §H6). "Continues exactly" (F4) exists
/// ONLY for same-account native resume.
#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, serde::Serialize, serde::Deserialize,
)]
pub enum FidelityLabel {
    /// Briefing pack (T4): task/decisions/TODOs/diffs only.
    F0,
    /// Full story, fresh start (T3 replay).
    F1,
    /// Same conversation, new account (compacted).
    F2,
    /// Same conversation, new account (T2 transplant).
    F3,
    /// Continues exactly (T1 same-account resume ONLY).
    F4,
}

impl FidelityLabel {
    /// The user-facing label (deep/09 §H6, verbatim).
    #[must_use]
    pub fn user_label(self) -> &'static str {
        match self {
            Self::F0 => "Briefing pack",
            Self::F1 => "Full story, fresh start",
            Self::F2 => "Same conversation, new account (compacted)",
            Self::F3 => "Same conversation, new account",
            Self::F4 => "Continues exactly",
        }
    }

    /// The wire name (`F0`..`F4`).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::F0 => "F0",
            Self::F1 => "F1",
            Self::F2 => "F2",
            Self::F3 => "F3",
            Self::F4 => "F4",
        }
    }

    /// One level lower (AJ-E210 Req.4: a failing validation cell
    /// lowers the pair's advertised ceiling until fixed). F0 stays
    /// F0 — the floor is honest, never negative.
    #[must_use]
    pub fn lowered(self) -> Self {
        match self {
            Self::F0 | Self::F1 => Self::F0,
            Self::F2 => Self::F1,
            Self::F3 => Self::F2,
            Self::F4 => Self::F3,
        }
    }

    /// The numeric index (`F0`=0 .. `F4`=4) — the score input the
    /// ranking slot consumes (`index / 4`).
    #[must_use]
    pub fn wire_index(self) -> u8 {
        match self {
            Self::F0 => 0,
            Self::F1 => 1,
            Self::F2 => 2,
            Self::F3 => 3,
            Self::F4 => 4,
        }
    }
}

/// A cap that lowered the label (recorded in order of application;
/// feeds `state.json`'s `fidelity.caps_applied`).
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct AppliedCap {
    /// Machine name (`f4-requires-same-account-native-resume`,
    /// `compact-boundary`, `drop-in-chat|rules|git`,
    /// `unvalidated-pair`, `zcode-target`).
    pub cap: String,
    /// The label the cap lowered to.
    pub capped_at: FidelityLabel,
}

/// One carry-list row: artifact + disposition in words + why.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct CarryRow {
    /// The artifact.
    pub artifact: ArtifactKind,
    /// The disposition word (`carried` / `translated` / `rebuilt` /
    /// `stays` / `stays (no written mapping)`).
    pub word: String,
    /// Why (plain words; for untranslated mappings this names the
    /// unmapped keys — never a silent omission).
    pub reason: String,
}

/// The engine input for one ordered pair.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct FidelityInput {
    /// Source provider id.
    pub from_provider: String,
    /// Target provider id.
    pub to_provider: String,
    /// Per-artifact dispositions (missing entries score Drop —
    /// the honest default).
    pub dispositions: BTreeMap<ArtifactKind, Disposition>,
    /// The source transcript had a `compact_boundary`.
    pub compact_boundary: bool,
    /// Cap-gating flags (they gate caps together).
    pub flags: PairFlags,
    /// Unmapped MCP/rules keys BY NAME (Req.5 — listed, never
    /// silently dropped).
    pub unmapped_keys: Vec<String>,
}

/// The cap-gating flags for one ordered pair.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct PairFlags {
    /// The AJ-E210 validation cell for this ordered pair is green.
    pub validated: bool,
    /// Same account on both sides (F4 eligibility).
    pub same_account: bool,
    /// Mechanism is native resume (F4 eligibility).
    pub native_resume: bool,
}

/// The scored result for one ordered pair.
#[derive(Clone, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
pub struct PairFidelity {
    /// The ordered pair (`from→to`).
    pub pair: String,
    /// The weighted mean BEFORE caps (internal; the label is the UI).
    pub score: f64,
    /// The label the user sees (always ships with the carry-list).
    pub label: FidelityLabel,
    /// Caps applied, in order.
    pub caps: Vec<AppliedCap>,
    /// The carry-list (one row per artifact).
    pub carry_list: Vec<CarryRow>,
}

/// The weighted mean plus the carry-list rows (one per artifact;
/// unmapped keys named, never silently omitted — Req.5).
fn weighted_mean_and_carry_list(input: &FidelityInput) -> (f64, Vec<CarryRow>) {
    let mut score = 0.0_f64;
    let mut carry_list = Vec::new();
    for (kind, weight) in WEIGHTS {
        let disposition = input
            .dispositions
            .get(kind)
            .copied()
            .unwrap_or(Disposition::Drop);
        score += weight * disposition.score();
        let reason = if matches!(
            disposition,
            Disposition::Translate {
                untranslated: true,
                ..
            }
        ) {
            format!(
                "no written mapping{}",
                if input.unmapped_keys.is_empty() {
                    String::new()
                } else {
                    format!(" — unmapped: {}", input.unmapped_keys.join(", "))
                }
            )
        } else if matches!(
            disposition,
            Disposition::Translate {
                keys_dropped: true,
                ..
            }
        ) {
            format!("some keys do not map: {}", input.unmapped_keys.join(", "))
        } else {
            disposition.carry_word().to_owned()
        };
        carry_list.push(CarryRow {
            artifact: *kind,
            word: disposition.carry_word().to_owned(),
            reason,
        });
    }
    // Normalize −0.0 (an all-Drop mean reduces to 0.0; the +0.0
    // collapse is the workspace convention for float reductions).
    (score + 0.0, carry_list)
}

/// Scores one ordered pair: weighted mean → band → caps (monotonic
/// lowering). Pure; no I/O.
#[must_use]
pub fn score_pair(input: &FidelityInput) -> PairFidelity {
    let (score, carry_list) = weighted_mean_and_carry_list(input);

    // Band (pass2/02 §3.4).
    let mut label = if score >= 0.95 {
        FidelityLabel::F4
    } else if score >= 0.75 {
        FidelityLabel::F3
    } else if score >= 0.45 {
        FidelityLabel::F1
    } else {
        FidelityLabel::F0
    };
    let mut caps = Vec::new();

    // Cap 1: F4 requires same-account native resume (deep/09 §H6).
    if label == FidelityLabel::F4 && !(input.flags.same_account && input.flags.native_resume) {
        label = FidelityLabel::F3;
        caps.push(AppliedCap {
            cap: "f4-requires-same-account-native-resume".to_owned(),
            capped_at: FidelityLabel::F3,
        });
    }

    // Cap 2: a compaction boundary in the source caps at F2
    // (live pre-compact context does not survive).
    if input.compact_boundary && label > FidelityLabel::F2 {
        label = FidelityLabel::F2;
        caps.push(AppliedCap {
            cap: "compact-boundary".to_owned(),
            capped_at: FidelityLabel::F2,
        });
    }

    // Cap 3: a Drop in chat/rules/git caps at F1 — the work itself
    // does not fully carry.
    let critical_drop = [
        ArtifactKind::Chat,
        ArtifactKind::Rules,
        ArtifactKind::GitDiff,
    ]
    .iter()
    .any(|kind| {
        matches!(
            input.dispositions.get(kind),
            Some(
                Disposition::Drop
                    | Disposition::Translate {
                        untranslated: true,
                        ..
                    }
            ) | None
        )
    });
    if critical_drop && label > FidelityLabel::F1 {
        label = FidelityLabel::F1;
        caps.push(AppliedCap {
            cap: "drop-in-chat-rules-git".to_owned(),
            capped_at: FidelityLabel::F1,
        });
    }

    // Cap 4: an unvalidated pair (E210 cell not green) caps at F1
    // regardless of mean — the honesty mechanism.
    if !input.flags.validated && label > FidelityLabel::F1 {
        label = FidelityLabel::F1;
        caps.push(AppliedCap {
            cap: "unvalidated-pair".to_owned(),
            capped_at: FidelityLabel::F1,
        });
    }

    // Cap 5: a `zcode` TARGET caps at F1 source / F0 target until a
    // SQLite clone + verified prime path pass (SPEC:282, E210).
    if input.to_provider == "zcode" && label > FidelityLabel::F0 {
        label = FidelityLabel::F0;
        caps.push(AppliedCap {
            cap: "zcode-target".to_owned(),
            capped_at: FidelityLabel::F0,
        });
    }

    PairFidelity {
        pair: format!("{}→{}", input.from_provider, input.to_provider),
        score,
        label,
        caps,
        carry_list,
    }
}

/// The base disposition table for an ordered provider pair, exactly
/// as researched (pass2/02 §3.2–§3.3). `validated=false` everywhere
/// — AJ-E210 owns which cells flip to validated; nothing here
/// advertises proven fidelity. Unknown providers (Gemini CLI /
/// `OpenCode` / future) are pack-only F0 (SPEC:140 adapter-first
/// rule).
#[must_use]
pub fn base_dispositions(from: &str, to: &str) -> FidelityInput {
    let copy_all = |exceptions: &[(ArtifactKind, Disposition)]| {
        let mut map = BTreeMap::new();
        for (kind, weight) in WEIGHTS {
            let _ = weight;
            map.insert(
                *kind,
                exceptions
                    .iter()
                    .find(|(k, _)| k == kind)
                    .map_or(Disposition::Copy, |(_, d)| *d),
            );
        }
        map
    };
    // Todos default Drop until a fixture proves carriage (§H2).
    let todos_drop = (ArtifactKind::Todos, Disposition::Drop);

    let dispositions = match (from, to) {
        ("claude-code", "claude-code") => copy_all(&[todos_drop]),
        ("codex", "codex") => copy_all(&[
            todos_drop,
            (ArtifactKind::Subagents, Disposition::Drop), // thread children unverified
        ]),
        ("zcode", "zcode") => copy_all(&[
            // Chat is BLOCKED today (SQLite clone, schema-coupled) —
            // honest Drop until E210 proves the clone.
            (ArtifactKind::Chat, Disposition::Drop),
            todos_drop,
            (ArtifactKind::Subagents, Disposition::Drop),
        ]),
        ("claude-code", "codex") | ("codex", "claude-code") => {
            cross_provider(&CrossPair::ClaudeCodex)
        }
        ("claude-code", "zcode") => cross_provider(&CrossPair::ClaudeZcode),
        ("zcode", "claude-code" | "codex") => cross_provider(&CrossPair::ZcodeOut),
        ("codex", "zcode") => cross_provider(&CrossPair::CodexZcode),
        // Any → Gemini CLI / OpenCode / future: pack-only (F0).
        _ => pack_only_dispositions(),
    };
    FidelityInput {
        from_provider: from.to_owned(),
        to_provider: to.to_owned(),
        dispositions,
        compact_boundary: false,
        flags: PairFlags::default(), // E210 owns flipping cells green
        unmapped_keys: Vec::new(),
    }
}

/// The researched cross-provider cell shapes (pass2/02 §3.3).
enum CrossPair {
    /// Claude ↔ Codex (both directions).
    ClaudeCodex,
    /// Claude to `zcode` (`zcode` is the target — F0 today).
    ClaudeZcode,
    /// `zcode` to Claude or `zcode` to Codex (`zcode` is the source).
    ZcodeOut,
    /// Codex to `zcode`.
    CodexZcode,
}

fn tr() -> Disposition {
    Disposition::Translate {
        keys_dropped: false,
        untranslated: false,
    }
}

fn cross_provider(pair: &CrossPair) -> BTreeMap<ArtifactKind, Disposition> {
    let mut map = BTreeMap::new();
    // Shared bones: chat/rules/mcp/env translate, git copies (shared
    // checkout), todos/subagents drop.
    map.insert(ArtifactKind::Chat, tr());
    map.insert(ArtifactKind::Rules, tr());
    map.insert(ArtifactKind::Mcp, tr());
    map.insert(ArtifactKind::GitDiff, Disposition::Copy);
    map.insert(ArtifactKind::EnvModel, tr());
    map.insert(ArtifactKind::Todos, Disposition::Drop);
    map.insert(ArtifactKind::Subagents, Disposition::Drop);
    match pair {
        CrossPair::ClaudeCodex => {
            // Skills translate where formats map; without a written
            // mapping they list as Drop (Req.5).
            map.insert(
                ArtifactKind::Skills,
                Disposition::Translate {
                    keys_dropped: false,
                    untranslated: true,
                },
            );
        }
        CrossPair::ClaudeZcode => {
            map.insert(ArtifactKind::Skills, Disposition::Copy); // where dirs align
        }
        CrossPair::ZcodeOut | CrossPair::CodexZcode => {
            if matches!(pair, CrossPair::ZcodeOut) {
                // AGENTS.md both sides for zcode→codex; translate via
                // the ring for zcode→claude.
                map.insert(ArtifactKind::Rules, tr());
            } else {
                map.insert(ArtifactKind::Rules, Disposition::Copy);
            }
            map.insert(ArtifactKind::Skills, Disposition::Drop);
        }
    }
    map
}

fn pack_only_dispositions() -> BTreeMap<ArtifactKind, Disposition> {
    let mut map = BTreeMap::new();
    map.insert(ArtifactKind::Chat, Disposition::Drop);
    // The rules ring still renders (AGENTS.md interchange).
    map.insert(ArtifactKind::Rules, tr());
    map.insert(ArtifactKind::Mcp, Disposition::Drop);
    map.insert(ArtifactKind::GitDiff, Disposition::Copy);
    map.insert(ArtifactKind::EnvModel, Disposition::Drop);
    map.insert(ArtifactKind::Todos, Disposition::Drop);
    map.insert(ArtifactKind::Skills, Disposition::Drop);
    map.insert(ArtifactKind::Subagents, Disposition::Drop);
    map
}

/// The full matrix over the researched ordered pairs (the golden
/// snapshot's source). Every cell starts unvalidated (D-04: ship in
/// beta, validate as a parallel workstream).
#[must_use]
pub fn full_matrix() -> Vec<PairFidelity> {
    let pairs = [
        ("claude-code", "claude-code"),
        ("codex", "codex"),
        ("zcode", "zcode"),
        ("claude-code", "codex"),
        ("codex", "claude-code"),
        ("claude-code", "zcode"),
        ("zcode", "claude-code"),
        ("codex", "zcode"),
        ("zcode", "codex"),
        ("claude-code", "gemini"),
        ("codex", "gemini"),
        ("zcode", "gemini"),
    ];
    pairs
        .iter()
        .map(|(from, to)| score_pair(&base_dispositions(from, to)))
        .collect()
}
