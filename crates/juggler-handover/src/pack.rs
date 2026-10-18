//! The canonical portable context pack v1 (AJ-E202; pass2/02 §4).
//!
//! ONE pack format serves as the T4 export, the staging artifact
//! for every edge (even native-resume/transplant edges generate a
//! manifest-only pack, so lineage and fidelity evidence always
//! exist), and the merge substrate. Generation is deterministic
//! from the input record: the same input produces byte-identical
//! `state.json`/`manifest.json` (LLM summaries, when they ever land,
//! write a separate `summary_llm.md` and never the canonical
//! record). Writes are atomic: temp dir → sha256 verify → rename; a
//! crash leaves the old pack or none, never a partial one.

use std::path::{Path, PathBuf};

use crate::manifest::{ManifestRow, credential_scan, row_for, sha256_hex};

/// The pack format version (`format_version: 1`).
pub const PACK_FORMAT_VERSION: u32 = 1;

/// One conversation turn in the neutral pack IR (callers map from
/// adapter transcripts — the pack never parses vendor formats).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct IrTurn {
    /// Message uuid, when the source format carries one.
    pub uuid: Option<String>,
    /// Who spoke.
    pub role: IrRole,
    /// The text content (tool noise already distilled by the caller).
    pub text: String,
    /// Epoch-ms of the turn, when known.
    pub at_ms: Option<i64>,
    /// Token usage for this turn, when the source recorded it.
    pub tokens: Option<TurnTokens>,
}

/// Role of an [`IrTurn`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum IrRole {
    /// Human/user turn.
    User,
    /// Model turn.
    Assistant,
}

/// Token counts for one turn (the replay-cost input).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct TurnTokens {
    /// Input tokens.
    pub input: u64,
    /// Output tokens.
    pub output: u64,
}

/// A Drop recorded on the carry-list (every Drop from the fidelity
/// matrix appears here AND in the manifest — never silently omitted).
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct DropEntry {
    /// The artifact that does not carry (e.g. `session todos`).
    pub artifact: String,
    /// Why (plain words — the carry-list copy).
    pub reason: String,
}

/// The neutral pack input: everything a pack records, already
/// resolved by the caller. Determinism contract: two calls with the
/// same input produce byte-identical output.
#[derive(Clone, PartialEq, Debug)]
pub struct PackInput {
    /// Pack identity (uuid; caller-generated once per pack).
    pub pack_id: String,
    /// Creation time (epoch-ms; part of the record — NOT `now()`).
    pub created_at_ms: i64,
    /// The work item this pack belongs to.
    pub work_id: String,
    /// The edge this pack stages, when in-switch (standalone exports
    /// have none).
    pub edge_id: Option<String>,
    /// Source identity.
    pub source: PackSource,
    /// Optional target hint (where the work is headed).
    pub target_hint: Option<PackTarget>,
    /// Project anchor.
    pub project: PackProject,
    /// Git state at freeze.
    pub git: PackGit,
    /// The conversation (ordered turns; empty for manifest-only).
    pub turns: Vec<IrTurn>,
    /// Whether the source had a compaction boundary.
    pub compacted: bool,
    /// Artifact lists (paths/rules/skill names — strings, not bytes).
    pub artifacts: PackArtifacts,
    /// Fidelity computed for this edge (AJ-E203).
    pub fidelity: PackFidelity,
    /// Quota at creation (honesty: what the source had left).
    pub quota_at_creation: Option<PackQuota>,
    /// Lineage: parent packs (two on merges) and parent edges.
    pub lineage: PackLineage,
    /// Extra staged file contents (`rel_path` like `diffs/x.patch`,
    /// `rules/AGENTS.md` → bytes); hashed into the manifest.
    pub staged_files: Vec<(String, Vec<u8>)>,
}

/// Source identity block.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct PackSource {
    /// Provider id (`claude-code`, …).
    pub provider: String,
    /// Account label (never the account's credentials).
    pub account_label: String,
    /// Launch recipe name, when launched under one.
    pub profile_name: Option<String>,
    /// The vendor-native session id.
    pub native_session_id: String,
    /// Harness version observed at freeze.
    pub harness_version: Option<String>,
    /// The source epoch (AJ-E201).
    pub epoch_id: String,
}

/// Target hint block.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct PackTarget {
    /// Provider id.
    pub provider: String,
    /// Account label.
    pub account_label: String,
}

/// Project block.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct PackProject {
    /// Canonical absolute root.
    pub root: String,
    /// AI-Juggler project id.
    pub project_id: String,
}

/// Git block.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct PackGit {
    /// Branch at freeze.
    pub branch: String,
    /// HEAD sha (or the honest absent marker).
    pub head_sha: Option<String>,
    /// Whether the worktree was dirty.
    pub dirty: bool,
    /// `diff --stat` one-liner, when dirty.
    pub diff_stat: Option<String>,
    /// Worktree path, when the source ran in a linked worktree.
    pub worktree_path: Option<String>,
}

/// Artifact inventory block (names/paths; bytes live in `diffs/`,
/// `rules/` and are hashed into the manifest).
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize, Default)]
pub struct PackArtifacts {
    /// Rule files carried (canonical AGENTS.md + renderings written).
    pub rules: Vec<String>,
    /// MCP config keys carried.
    pub mcp: Vec<String>,
    /// Skill/command names carried.
    pub skills: Vec<String>,
    /// Todo entries carried (empty until a fixture proves carriage —
    /// AJ-E203's default- Drop).
    pub todos: Vec<String>,
    /// Every Drop, with reasons.
    pub drops: Vec<DropEntry>,
}

/// Fidelity block (AJ-E203's output).
#[derive(Clone, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
pub struct PackFidelity {
    /// The F-label shown to the user (F0–F4).
    pub level: String,
    /// The capped composite score (internal; the label is the UI).
    pub score: f64,
    /// Caps applied (e.g. `drop-in-chat`, `unvalidated-pair`).
    pub caps_applied: Vec<String>,
}

/// Quota block at creation.
#[derive(Clone, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
pub struct PackQuota {
    /// Account label.
    pub account: String,
    /// Used percentage, when known.
    pub used_pct: Option<f64>,
    /// Confidence tier of the reading.
    pub confidence: String,
}

/// Lineage block (merges list two parents).
#[derive(Clone, PartialEq, Eq, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct PackLineage {
    /// Parent pack ids (0, 1, or 2 — 2 on merges).
    pub parent_pack_ids: Vec<String>,
    /// Parent edge ids.
    pub parent_edge_ids: Vec<String>,
}

/// `state.json` — the canonical record. Field order is the wire
/// contract (deterministic serialization); readers IGNORE unknown
/// fields (`#[serde(default)]` + no `deny_unknown_fields`).
#[derive(Clone, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
pub struct PackState {
    /// Always `1` for this format.
    #[serde(default)]
    pub format_version: u32,
    /// Pack uuid.
    pub pack_id: String,
    /// Creation epoch-ms.
    pub created_at: i64,
    /// Work item id.
    pub work_id: String,
    /// Staged edge id (absent for standalone exports).
    #[serde(default)]
    pub edge_id: Option<String>,
    /// Source identity.
    pub source: PackSource,
    /// Target hint.
    #[serde(default)]
    pub target_hint: Option<PackTarget>,
    /// Project anchor.
    pub project: PackProject,
    /// Git state.
    pub git: PackGit,
    /// Conversation facts.
    pub conversation: PackConversation,
    /// Artifact inventory (drops included — never silently omitted).
    pub artifacts: PackArtifacts,
    /// Fidelity computed for the edge.
    pub fidelity: PackFidelity,
    /// Quota at creation.
    #[serde(default)]
    pub quota_at_creation: Option<PackQuota>,
    /// Lineage (two parents on merges).
    #[serde(default)]
    pub lineage: PackLineage,
    /// Redaction counts (reserved: `[]` until AJ-E208; values never
    /// stored).
    #[serde(default)]
    pub redactions: Vec<RedactionCount>,
}

/// Conversation facts block.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct PackConversation {
    /// Number of turns.
    pub turn_count: u32,
    /// sha256 over the ordered message uuids (one per line).
    pub message_uuids_sha256: String,
    /// Whether the source had a compaction boundary.
    pub compacted: bool,
    /// First turn epoch-ms, when known.
    pub first_turn_at: Option<i64>,
    /// Last turn epoch-ms, when known.
    pub last_turn_at: Option<i64>,
    /// Estimated replay tokens (sum of recorded turn tokens; the
    /// honest measured floor).
    pub replay_tokens_est: u64,
}

/// One redaction count (kind + count only; values never stored,
/// logged, or written outside the pack) — AJ-E208.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct RedactionCount {
    /// Detection kind (`api-key`, `bearer`, …).
    pub kind: String,
    /// How many were redacted.
    pub count: u32,
}

/// Everything a generation pass produces: the two canonical files'
/// bytes, the human brief, and the manifest rows (caller persists).
#[derive(Clone, PartialEq, Debug)]
pub struct GeneratedPack {
    /// `state.json` bytes.
    pub state_json: Vec<u8>,
    /// `manifest.json` bytes.
    pub manifest_json: Vec<u8>,
    /// `PACK.md` bytes (the human brief — never raw JSON as the only
    /// surface).
    pub pack_md: Vec<u8>,
    /// `conversation.md` bytes (full replay text or distilled).
    pub conversation_md: Vec<u8>,
    /// Manifest rows for every file the pack ships (drops included,
    /// with `disposition: "drop"`).
    pub manifest_rows: Vec<ManifestRow>,
    /// Extra file contents to stage under `diffs/` and `rules/`
    /// (`rel_path` → bytes), hashed into the manifest.
    pub extra_files: Vec<(String, Vec<u8>)>,
}

/// Generation mode for `conversation.md`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ConversationMode {
    /// T3: the full replay text.
    Full,
    /// T4: distilled summary + the last N turns.
    Distilled {
        /// How many trailing turns render verbatim.
        last_n: u32,
    },
}

/// Deterministically generates the pack content from `input`.
/// Same input ⇒ byte-identical output (the determinism contract;
/// pinned by golden tests). Pure — no I/O, no clock, no randomness.
#[must_use]
pub fn generate_pack(input: &PackInput, mode: ConversationMode) -> GeneratedPack {
    generate_pack_with_redactions(input, mode, Vec::new())
}

/// Same as [`generate_pack`] with the redaction counts recorded in
/// the manifest state (AJ-E208 — rows must be set BEFORE
/// serialization so the manifest hashes the final bytes).
#[must_use]
pub(crate) fn generate_pack_with_redactions(
    input: &PackInput,
    mode: ConversationMode,
    redactions: Vec<RedactionCount>,
) -> GeneratedPack {
    // Conversation facts (deterministic reductions over the turns).
    let mut uuids = String::new();
    let mut replay_tokens = 0_u64;
    for turn in &input.turns {
        if let Some(uuid) = &turn.uuid {
            uuids.push_str(uuid);
            uuids.push('\n');
        }
        if let Some(tokens) = turn.tokens {
            replay_tokens = replay_tokens
                .saturating_add(tokens.input)
                .saturating_add(tokens.output);
        }
    }
    let conversation = PackConversation {
        turn_count: u32::try_from(input.turns.len()).unwrap_or(u32::MAX),
        message_uuids_sha256: sha256_hex(uuids.as_bytes()),
        compacted: input.compacted,
        first_turn_at: input.turns.iter().find_map(|turn| turn.at_ms),
        last_turn_at: input.turns.iter().rev().find_map(|turn| turn.at_ms),
        replay_tokens_est: replay_tokens,
    };
    let state = PackState {
        format_version: PACK_FORMAT_VERSION,
        pack_id: input.pack_id.clone(),
        created_at: input.created_at_ms,
        work_id: input.work_id.clone(),
        edge_id: input.edge_id.clone(),
        source: input.source.clone(),
        target_hint: input.target_hint.clone(),
        project: input.project.clone(),
        git: input.git.clone(),
        conversation,
        artifacts: input.artifacts.clone(),
        fidelity: input.fidelity.clone(),
        quota_at_creation: input.quota_at_creation.clone(),
        lineage: input.lineage.clone(),
        redactions, // AJ-E208: counts under the user's policy
    };
    let state_json = serde_json::to_vec_pretty(&state).unwrap_or_default();
    let pack_md = render_pack_md(&state);
    let conversation_md = render_conversation_md(&state, &input.turns, mode);

    // The manifest: every shipped file + every Drop as an explicit
    // row (drops never silently omitted).
    let mut manifest_rows = vec![
        row_for("PACK.md", &pack_md, "brief", "regenerate"),
        row_for("conversation.md", &conversation_md, "conversation", "copy"),
        row_for("state.json", &state_json, "state", "copy"),
    ];
    for (rel_path, bytes) in &input.extra_staged_files() {
        manifest_rows.push(row_for(rel_path, bytes, staged_kind(rel_path), "copy"));
    }
    for drop in &input.artifacts.drops {
        // A Drop row: no bytes, disposition drop, reason carried in
        // state.json's carry-list.
        manifest_rows.push(ManifestRow {
            rel_path: format!("drops/{}", slug(&drop.artifact)),
            sha256: sha256_hex(drop.reason.as_bytes()),
            bytes: u64::try_from(drop.reason.len()).unwrap_or(u64::MAX),
            kind: "drop".to_owned(),
            disposition: "drop".to_owned(),
        });
    }
    let manifest_json = serde_json::to_vec_pretty(&manifest_rows).unwrap_or_default();

    GeneratedPack {
        state_json,
        manifest_json,
        pack_md,
        conversation_md,
        manifest_rows,
        extra_files: input.extra_staged_files(),
    }
}

impl PackInput {
    /// The staged extra files (diffs + rules) the caller attached.
    fn extra_staged_files(&self) -> Vec<(String, Vec<u8>)> {
        self.staged_files.clone()
    }
}

/// Where an extra staged file belongs by extension.
fn staged_kind(rel_path: &str) -> &'static str {
    if rel_path.starts_with("diffs/") {
        "diffs"
    } else if rel_path.starts_with("rules/") {
        "rules"
    } else {
        "artifact"
    }
}

/// Lowercases and de-spaces an artifact name for a drop row path.
fn slug(raw: &str) -> String {
    raw.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_lowercase()
}

/// Renders `PACK.md`: task, constraints, decisions, open todos, next
/// steps — the "What travels / What stays" brief (SPEC §7.3; deep/09
/// §H5 "the pack is the preview"). No tier codes (T1–T4 stay
/// engine-internal per deep/09 D2).
fn render_pack_md(state: &PackState) -> Vec<u8> {
    use std::fmt::Write as _;
    let mut out = String::new();
    let _ = writeln!(
        out,
        "# Context pack\n\nWork `{}` on `{}` — {} under {}.",
        state.work_id, state.project.root, state.source.account_label, state.source.provider
    );
    let _ = writeln!(out, "\n## What travels\n");
    let _ = writeln!(
        out,
        "- The conversation: {} turns ({})",
        state.conversation.turn_count,
        if state.conversation.compacted {
            "with a compaction boundary"
        } else {
            "uncompacted"
        }
    );
    if !state.artifacts.rules.is_empty() {
        let _ = writeln!(out, "- Rules: {}", state.artifacts.rules.join(", "));
    }
    if !state.artifacts.mcp.is_empty() {
        let _ = writeln!(out, "- MCP config keys: {}", state.artifacts.mcp.join(", "));
    }
    if !state.artifacts.skills.is_empty() {
        let _ = writeln!(
            out,
            "- Skills/commands: {}",
            state.artifacts.skills.join(", ")
        );
    }
    if !state.artifacts.todos.is_empty() {
        let _ = writeln!(out, "- Todos: {}", state.artifacts.todos.join(", "));
    }
    let _ = writeln!(
        out,
        "- Git: branch {}{}",
        state.git.branch,
        if state.git.dirty {
            " (dirty — diff snapshotted)"
        } else {
            ""
        }
    );
    let _ = writeln!(out, "\n## What stays\n");
    if state.artifacts.drops.is_empty() {
        let _ = writeln!(out, "- Nothing known drops on this switch.");
    } else {
        for drop in &state.artifacts.drops {
            let _ = writeln!(out, "- {}: {}", drop.artifact, drop.reason);
        }
    }
    let _ = writeln!(out, "\n## Next steps\n");
    let _ = writeln!(
        out,
        "- Fidelity of this switch: {} (beta)",
        state.fidelity.level
    );
    let _ = writeln!(
        out,
        "- Replay estimate: ≈{} tokens",
        state.conversation.replay_tokens_est
    );
    let _ = writeln!(
        out,
        "- Pack `{}` created {}",
        state.pack_id, state.created_at
    );
    out.into_bytes()
}

fn render_conversation_md(state: &PackState, turns: &[IrTurn], mode: ConversationMode) -> Vec<u8> {
    use std::fmt::Write as _;
    let _ = state;
    let mut out = String::new();
    let _ = writeln!(out, "# Conversation");
    let rendered: &[IrTurn] = match mode {
        ConversationMode::Full => turns,
        ConversationMode::Distilled { last_n } => {
            let keep = usize::try_from(last_n).unwrap_or(0).min(turns.len());
            let start = turns.len().saturating_sub(keep);
            if start > 0 {
                let _ = writeln!(
                    out,
                    "\n(distilled — {start} earlier turns summarized by their presence here; last {keep} verbatim)"
                );
            }
            &turns[start..]
        }
    };
    for turn in rendered {
        let who = match turn.role {
            IrRole::User => "User",
            IrRole::Assistant => "Assistant",
        };
        let _ = writeln!(out, "\n## {who}\n\n{}", turn.text.trim_end());
    }
    out.into_bytes()
}

/// A pack write error (plain-language messages reach the user).
#[derive(Debug, thiserror::Error)]
pub enum PackWriteError {
    /// Credential material was found in pack content — the write is
    /// refused before any byte reaches disk.
    #[error("pack content contains credential material ({0} hits) — refused")]
    Credentials(usize),
    /// An I/O failure (temp write, verify, or rename).
    #[error("pack write failed: {0}")]
    Io(#[from] std::io::Error),
    /// Verification failed: staged bytes do not match the manifest.
    #[error("pack verification failed for {0}")]
    Verify(String),
}

/// Writes the pack atomically: a temp sibling directory is fully
/// written and sha256-verified against the manifest, then renamed
/// into place. A crash leaves the previous pack (or none) — never a
/// partial directory. The credential guard runs BEFORE any write:
/// no pack file may ship credential material under any policy.
///
/// # Errors
///
/// See [`PackWriteError`] — credentials, I/O, or verification.
pub fn write_pack(dir: &Path, pack: &GeneratedPack) -> Result<(), PackWriteError> {
    // The always-on guard: every file the pack would ship.
    let mut hits = credential_scan(&pack.pack_md).len();
    hits += credential_scan(&pack.conversation_md).len();
    hits += credential_scan(&pack.state_json).len();
    for (_, bytes) in &pack.extra_files {
        hits += credential_scan(bytes).len();
    }
    if hits > 0 {
        return Err(PackWriteError::Credentials(hits));
    }

    let parent = dir.parent().unwrap_or_else(|| Path::new("."));
    let tmp: PathBuf = parent.join(format!(
        ".{}.tmp-{}",
        dir.file_name()
            .map_or_else(|| "pack".into(), |n| n.to_string_lossy().into_owned()),
        std::process::id()
    ));
    if tmp.exists() {
        std::fs::remove_dir_all(&tmp)?;
    }
    std::fs::create_dir_all(&tmp)?;
    std::fs::write(tmp.join("PACK.md"), &pack.pack_md)?;
    std::fs::write(tmp.join("conversation.md"), &pack.conversation_md)?;
    std::fs::write(tmp.join("state.json"), &pack.state_json)?;
    std::fs::write(tmp.join("manifest.json"), &pack.manifest_json)?;
    for (rel_path, bytes) in &pack.extra_files {
        let target = tmp.join(rel_path);
        if let Some(folder) = target.parent() {
            std::fs::create_dir_all(folder)?;
        }
        std::fs::write(target, bytes)?;
    }
    // The EMPTY marker when no diffs travelled.
    if pack
        .extra_files
        .iter()
        .all(|(rel, _)| !rel.starts_with("diffs/"))
    {
        // `fs::write` creates no directories — a diff-less pack never
        // materialised `diffs/` via the extra-files loop above.
        std::fs::create_dir_all(tmp.join("diffs"))?;
        std::fs::write(tmp.join("diffs/EMPTY"), b"no uncommitted diff at freeze\n")?;
    }

    // sha256 verify BEFORE the rename — a partial or corrupted temp
    // never becomes the pack.
    for row in &pack.manifest_rows {
        if row.disposition == "drop" {
            continue; // drop rows carry reasons, not staged bytes
        }
        let bytes = std::fs::read(tmp.join(&row.rel_path)).map_err(PackWriteError::Io)?;
        if crate::manifest::sha256_hex(&bytes) != row.sha256 {
            let _ = std::fs::remove_dir_all(&tmp);
            return Err(PackWriteError::Verify(row.rel_path.clone()));
        }
    }
    if dir.exists() {
        std::fs::remove_dir_all(dir)?;
    }
    std::fs::rename(&tmp, dir)?;
    Ok(())
}
