//! AJ-E3013: what the window is told about AI Juggler's connector in a
//! desktop App, and about a pool's desktop members.

use serde::Serialize;
use ts_rs::TS;

use crate::features::switch_setup::GetItInfo;

/// Where the connector stands in one desktop App.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ConnectorStatusInfo {
    /// The App (`apps.id`).
    pub app_id: String,
    /// `absent` | `current` (AI Juggler's, as this edition writes it) |
    /// `outdated` (an older edition's: Add updates it) | `changed` (changed
    /// by someone, or something AI Juggler didn't write) | `unavailable`
    /// (it can't be added here: `why_not` says why).
    pub state: String,
    /// The files changed by someone, inside the App's config folder.
    pub changed: Vec<String>,
    /// Why it can't be added here, for `unavailable`, in words for logs
    /// (the window words `unavailable` instead).
    pub why_not: Option<String>,
    /// Why there's no connector to use here, a code for the window to word:
    /// `missing` (no juggler-agent beside AI Juggler) | `unsafe_location`
    /// (it's in a folder others own or can write: `unsafe_folder`) |
    /// `not_offered` (not for this App, or not yet: the Codex app until lab
    /// probe C2) | `unknown_app` | `no_folder` (the App has no config
    /// folder AI Juggler knows yet) | `no_connector` (offered here, not
    /// added: the `absent` state). `null` when it's there.
    #[ts(optional = nullable)]
    pub unavailable: Option<String>,
    /// With `unsafe_location`: the folder that failed the check, the home
    /// folder written `~`.
    #[ts(optional = nullable)]
    pub unsafe_folder: Option<String>,
    /// The App's own config folder it goes in.
    pub folder: Option<String>,
    /// When the App's sessions pick it up, in a sentence.
    pub picked_up: Option<String>,
    /// Whether AI Juggler can confirm which session of this App calls
    /// (a Codex app thread: not yet), so its sessions join as members.
    pub attests: bool,
    /// Whether this App's sessions can be woken by a pool message (D-119).
    pub can_wake: bool,
}

/// One file in the preview.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ConnectorFileInfo {
    /// The file.
    pub path: String,
    /// `create` | `replace` (AI Juggler's own, an older edition's) |
    /// `merge` (one table into the tool's own file) | `same` (nothing to
    /// write) | `refuse` (something AI Juggler didn't write is there).
    pub action: String,
    /// The file as it is (`null`: not there).
    pub before: Option<String>,
    /// The file as Add leaves it.
    pub after: String,
}

/// What Add would write, before it does.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ConnectorPreviewInfo {
    /// Where it stands now.
    pub status: ConnectorStatusInfo,
    /// Each file, before and after.
    pub files: Vec<ConnectorFileInfo>,
    /// Why Add would refuse, when it would.
    pub refused: Option<String>,
    /// The guide: how the user adds it themselves instead.
    pub guide: Option<GetItInfo>,
}

/// What Remove did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ConnectorRemovalInfo {
    /// Where it stands now.
    pub status: ConnectorStatusInfo,
    /// The files it took AI Juggler's parts out of.
    pub removed: Vec<String>,
    /// The files changed by someone since Add, left as they are.
    pub kept: Vec<String>,
}

/// AJ-E3013 (the lab's A6/A9): one desktop session a pool holds that
/// reads its messages through AI Juggler's connector, for the pool's
/// member and recipient lists beside its sessions AI Juggler runs (which
/// come from `pools_members`): the Messages "To" list and the diagram's
/// member nodes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct DesktopSessionInfo {
    /// What a message to it takes as `to` (`conversation:<folder>/<id>`),
    /// and its node's key on the diagram.
    pub member: String,
    /// Its attested session's id (`desktop:<folder>/<id>`) while one of its
    /// calls is attested (the last ten minutes); `null` for one whose App
    /// has the connector but that hasn't called lately.
    pub session_id: Option<String>,
    /// Its conversation's title, else "A session in another app".
    pub name: String,
    /// Where it runs: its App, in words ("Claude app · Work").
    pub place: Option<String>,
    /// Its tool, in words.
    pub tool: Option<String>,
    /// Its account's label, when the conversation index knows it.
    pub account: Option<String>,
    /// Its role in the pool (`lead`, `member`, `reviewer`, `observer`).
    pub role: String,
    /// Its desktop App (`apps.id`), when known.
    pub app_id: Option<String>,
    /// How it's in the pool: `conversation` (named on its own) or the broad
    /// member that brought it in (`app`, `account`, `tool`, `mac`,
    /// `everyone`).
    pub via: String,
    /// `wakes` (a new pool message wakes it: D-119) | `next_turn` (it reads
    /// it at its next turn).
    pub wake: String,
}

/// A pool's desktop member, for the canvas (D-116, D-119).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct DesktopMemberInfo {
    /// Its stored ref (`conversation:<folder>/<id>` or `app:<id>`).
    pub member: String,
    /// Its desktop App, when known.
    pub app_id: Option<String>,
    /// Its App's connector: as [`ConnectorStatusInfo::state`].
    pub connector: String,
    /// Whether one of its sessions called through the connector lately
    /// (its first confirmed call makes it connected).
    pub connected: bool,
    /// `wakes` (a new pool message starts its turn: the pool at Full, or a
    /// rule the user made) | `next_turn` (it reads them on its next turn)
    /// | `cant_wake` (its app has no way in: a Codex app thread).
    pub wake: String,
}
