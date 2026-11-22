//! AJ-E1610 (D-66, D-105, D-106): what a tool's own desktop app allows,
//! route by route, as data beside its [`crate::live::HostApp`].
//!
//! Every app is an App the user may add, open, keep alive and bring into
//! focus (the places book does that for any app the host registry knows).
//! What differs per app is data here:
//!
//! - **Routes**: each link or adapter route into the app, with how sure it
//!   is ([`RouteStatus`]). Only a documented route, or one a live check
//!   proved, is ever used; an unverified one stays a declared capability
//!   and the caller falls back to plain focus.
//! - **Track-only parts**: what has no sound route, with one plain line
//!   saying why, which the App's page shows as it is.
//! - **Driving**: a tool whose own guideline bars driving it on consumer
//!   plans (Antigravity, D-105's sub-section) says so here, so no runtime
//!   and no automatic switch is ever started for such an account.
//!
//! Must not know any one tool: the facts are each adapter's data.

/// Where an app's facts come from.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Seen {
    /// Read from the installed bundle on a real Mac.
    Installed,
    /// From the vendor's docs or source only: confirm on a real install.
    #[default]
    Source,
}

/// What a route does in the app.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteAction {
    /// A new chat.
    NewChat,
    /// One chat by its id (`{id}`).
    OpenChat,
    /// A new coding session in a folder (`{folder}`).
    NewInFolder,
    /// One of the app's own sessions by its id (`{id}`).
    OpenSession,
    /// The app's session `{id}` continued where it was.
    ContinueSession,
    /// The app's list of sessions waiting for the user.
    NeedsInput,
    /// Read its sessions (through the tool's adapter, from its own files).
    ReadSessions,
    /// Reply into one of its sessions (through the tool's adapter).
    Reply,
}

/// How sure a route is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteStatus {
    /// The vendor documents it.
    Documented,
    /// Undocumented, but a live check proved it (the evidence names it).
    LiveChecked,
    /// Found in the app's own files or a second-hand source, never proved:
    /// declared, never used (plain focus instead) until a live check.
    Unverified,
}

impl RouteStatus {
    /// Whether AI Juggler may use it.
    #[must_use]
    pub fn usable(self) -> bool {
        matches!(self, Self::Documented | Self::LiveChecked)
    }
}

/// One route into the app.
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct AppRoute {
    /// What it does.
    pub action: RouteAction,
    /// The link, with `{id}` or `{folder}`; `None` for a route through
    /// the tool's adapter (its own files, its own server).
    #[serde(default)]
    pub link: Option<String>,
    /// How sure it is.
    pub status: RouteStatus,
    /// Where it is written down (a docs page, a research note's section).
    pub source: String,
}

/// A part of the app with no sound route.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TrackPart {
    /// Opening it for one account (a profile per account).
    Account,
    /// Opening one given session.
    SessionLink,
    /// Listing or reading its sessions.
    Sessions,
    /// Replying into its sessions from AI Juggler.
    Reply,
}

/// One track-only part, and why, in one plain line.
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct TrackOnly {
    /// The part.
    pub part: TrackPart,
    /// Why, in plain words, as the App's page says it.
    pub why: String,
}

/// Whether the tool may be driven (a runtime of its own, automatic
/// switching) for an account.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Driving {
    /// As every tool (D-105).
    #[default]
    Allowed,
    /// The tool's own guideline bars it on consumer plans: open, keep
    /// alive, focus and read only; never a driven runtime, never an
    /// automatic switch onto or off such an account.
    NotOnConsumerPlans,
}

/// A desktop app's facts beyond how it is found and linked.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Deserialize)]
pub struct DesktopFacts {
    /// Its tool's id, when no adapter of the registry describes the app
    /// yet (Cursor, Kiro, Antigravity before their adapters).
    #[serde(default)]
    pub tool: Option<String>,
    /// An editor that is also an AI app the user runs: an App (offered,
    /// opened, kept alive, focused), while a session in its terminal keeps
    /// the editor's Go to it.
    #[serde(default)]
    pub editor_app: bool,
    /// Other bundle ids of the same app (beta and dev builds).
    #[serde(default)]
    pub other_bundle_ids: Vec<String>,
    /// Where its facts come from.
    #[serde(default)]
    pub seen: Seen,
    /// For an app with no published bundle id: a folder under the home
    /// folder whose presence says it is installed (its own state folder).
    /// Looked at, never read inside, never written.
    #[serde(default)]
    pub install_folder: Option<String>,
    /// Its routes.
    #[serde(default)]
    pub routes: Vec<AppRoute>,
    /// Its parts with no sound route.
    #[serde(default)]
    pub track_only: Vec<TrackOnly>,
    /// Whether it may be driven.
    #[serde(default)]
    pub driving: Driving,
}

impl DesktopFacts {
    /// The route for `action`, sure or not.
    #[must_use]
    pub fn route(&self, action: RouteAction) -> Option<&AppRoute> {
        self.routes.iter().find(|route| route.action == action)
    }

    /// The route for `action` AI Juggler may use.
    #[must_use]
    pub fn usable(&self, action: RouteAction) -> Option<&AppRoute> {
        self.route(action).filter(|route| route.status.usable())
    }

    /// Why `part` is track-only, when it is.
    #[must_use]
    pub fn why_not(&self, part: TrackPart) -> Option<&str> {
        self.track_only
            .iter()
            .find(|track| track.part == part)
            .map(|track| track.why.as_str())
    }

    /// Whether a runtime of AI Juggler's own may drive the tool for an
    /// account (`consumer`: the account is on a consumer plan, or its plan
    /// isn't known yet, which counts as one).
    #[must_use]
    pub fn may_drive(&self, consumer: bool) -> bool {
        !(consumer && self.driving == Driving::NotOnConsumerPlans)
    }

    /// Whether AI Juggler may switch onto or off an account by itself
    /// (a switch the user starts by hand always stays).
    #[must_use]
    pub fn may_switch_automatically(&self, consumer: bool) -> bool {
        self.may_drive(consumer)
    }
}

/// `folder` for a link: each byte outside the plain set as `%XX`.
#[must_use]
pub fn link_escape(text: &str) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || b"/-._~".contains(&byte) {
            out.push(char::from(byte));
        } else {
            let _ = write!(out, "%{byte:02X}");
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts() -> DesktopFacts {
        serde_json::from_str(
            r#"{
              "routes": [
                {"action": "new_chat", "link": "x://new", "status": "documented", "source": "docs"},
                {"action": "continue_session", "link": "x://c/{id}", "status": "unverified", "source": "bundle"},
                {"action": "open_session", "link": "x://s/{id}", "status": "live_checked", "source": "AJ-E2102"}
              ],
              "track_only": [{"part": "reply", "why": "No way in."}],
              "driving": "not_on_consumer_plans"
            }"#,
        )
        .unwrap()
    }

    #[test]
    fn only_a_documented_or_live_checked_route_is_usable() {
        let facts = facts();
        assert!(facts.usable(RouteAction::NewChat).is_some());
        assert!(facts.usable(RouteAction::OpenSession).is_some());
        assert!(facts.route(RouteAction::ContinueSession).is_some());
        assert!(facts.usable(RouteAction::ContinueSession).is_none());
        assert!(facts.usable(RouteAction::Reply).is_none());
        assert_eq!(facts.why_not(TrackPart::Reply), Some("No way in."));
        assert_eq!(facts.why_not(TrackPart::Account), None);
    }

    #[test]
    fn a_consumer_plan_is_never_driven_nor_switched_automatically_where_the_tool_says_so() {
        let facts = facts();
        assert!(!facts.may_drive(true));
        assert!(!facts.may_switch_automatically(true));
        assert!(facts.may_drive(false));
        let plain = DesktopFacts::default();
        assert!(plain.may_drive(true) && plain.may_switch_automatically(true));
    }

    #[test]
    fn a_folder_is_escaped_for_a_link() {
        assert_eq!(link_escape("/Users/you/my app"), "/Users/you/my%20app");
        assert_eq!(link_escape("/a&b?c"), "/a%26b%3Fc");
    }
}
