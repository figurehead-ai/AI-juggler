//! AJ-E1610 (D-66, D-105, D-106): what an App's desktop app allows from
//! AI Juggler, from its adapter's data (`juggler_adapter::desktop_app`).
//!
//! - [`info`]: its routes and how sure each is, what opens here by the
//!   app's own documented links, and each track-only part with its one
//!   line, for the App's page.
//! - [`open_route`]: "Open here": one of those links (a new chat, a new
//!   session in a folder), sent to the running profile's own process,
//!   never the app at large. A route that isn't documented or proved live
//!   is never sent: it is declared, and the page says what works instead.
//!
//! Nothing here types into an app, clicks in it, reads its sign-in or
//! writes its folders: a link is the app's own door, and the app decides
//! (the Claude app asks before it opens a folder).

use std::path::Path;
use std::sync::Arc;

use juggler_adapter::desktop_app::{RouteAction, RouteStatus, Seen};
use juggler_adapter::live::HostApp;

use super::{NOT_ADDED, outcome, running_now, with_book};
use crate::error::{AppError, AppResult};
use crate::features::conversations::dto::GoToOutcomeInfo;
use crate::features::conversations::goto::{Desktop, LinkSent};
use crate::features::conversations::places::HostRegistry;
use crate::state::AppState;

/// What "Open here" may send: a new chat, a new session in a folder.
pub const OPEN_HERE: [RouteAction; 2] = [RouteAction::NewChat, RouteAction::NewInFolder];

/// One route into the app, and how sure it is.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct RouteInfo {
    /// `new_chat`, `open_chat`, `new_in_folder`, `open_session`,
    /// `continue_session`, `needs_input`, `read_sessions` or `reply`.
    pub action: String,
    /// `documented`, `live_checked` or `unverified` (declared, never used:
    /// plain focus instead until a live check).
    pub status: String,
    /// Whether AI Juggler uses it.
    pub usable: bool,
}

/// One part with no sound route, and why.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct TrackOnlyInfo {
    /// `account`, `session_link`, `sessions` or `reply`.
    pub part: String,
    /// Why, in one plain line.
    pub why: String,
}

/// What an App's desktop app allows from AI Juggler.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct AppReachInfo {
    /// `installed` (its facts were read from a real install) or `source`
    /// (from its docs and source, to be confirmed on a real install).
    pub seen: String,
    /// What opens here by its own link ([`OPEN_HERE`] it may send).
    pub open_here: Vec<String>,
    /// Each route, and how sure it is.
    pub routes: Vec<RouteInfo>,
    /// Each track-only part, with its line.
    pub track_only: Vec<TrackOnlyInfo>,
}

/// A value's word as the data names it (`new_chat`).
fn word<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_default()
}

/// What `host` allows; `None` for an app that declares nothing (a
/// terminal, an editor that isn't an AI app).
#[must_use]
pub fn info(host: &HostApp) -> Option<AppReachInfo> {
    let facts = &host.desktop;
    if facts.routes.is_empty() && facts.track_only.is_empty() {
        return None;
    }
    Some(AppReachInfo {
        seen: match facts.seen {
            Seen::Installed => "installed",
            Seen::Source => "source",
        }
        .to_owned(),
        open_here: OPEN_HERE
            .iter()
            .filter(|action| {
                facts
                    .usable(**action)
                    .is_some_and(|route| route.link.is_some())
            })
            .map(word)
            .collect(),
        routes: facts
            .routes
            .iter()
            .map(|route| RouteInfo {
                action: word(&route.action),
                status: word(&route.status),
                usable: route.status.usable(),
            })
            .collect(),
        track_only: facts
            .track_only
            .iter()
            .map(|track| TrackOnlyInfo {
                part: word(&track.part),
                why: track.why.clone(),
            })
            .collect(),
    })
}

/// The action named `word` (`new_chat`), when "Open here" may send it.
fn open_here_action(word: &str) -> Option<RouteAction> {
    let action: RouteAction = serde_json::from_value(serde_json::Value::from(word)).ok()?;
    OPEN_HERE.contains(&action).then_some(action)
}

/// "Open here": the app's own link for `action` (`new_chat`,
/// `new_in_folder` with `folder`), sent to place `id`'s running process.
/// macOS is never let ask here (the link goes quietly, as Go to it's).
///
/// # Errors
///
/// [`AppError::Refused`], in words: an App the user hasn't added, an
/// action "Open here" doesn't send, a route the app doesn't have or that
/// isn't proved (its track-only line when it has one), or the app isn't
/// running.
pub fn open_route(
    state: &Arc<AppState>,
    id: &str,
    action: &str,
    folder: Option<&str>,
    desktop: &dyn Desktop,
) -> AppResult<GoToOutcomeInfo> {
    let place = with_book(state, |book| {
        book.places.get(id).filter(|place| place.users).cloned()
    })?
    .ok_or_else(|| AppError::Refused(NOT_ADDED.to_owned()))?;
    let label = super::named(state, &place);
    let action = open_here_action(action)
        .ok_or_else(|| AppError::Refused("AI Juggler doesn't open that here".to_owned()))?;
    let hosts = HostRegistry::of(&state.registry);
    let host = hosts
        .by_id(&place.app_id)
        .ok_or_else(|| AppError::Refused(format!("AI Juggler doesn't know {label} any more")))?;
    let folder = folder.map(Path::new);
    let Some(link) = host.route_link(action, None, folder) else {
        let unproved = host
            .desktop
            .route(action)
            .is_some_and(|route| route.status == RouteStatus::Unverified);
        return Err(AppError::Refused(if unproved {
            format!("{label}'s link for that isn't proved yet; Go to it instead")
        } else if action == RouteAction::NewInFolder && folder.is_some_and(|f| !f.is_absolute()) {
            "That folder isn't a full path".to_owned()
        } else {
            format!("{label} has no link for that; Go to it instead")
        }));
    };
    let Some((pid, _)) = running_now(state, id) else {
        return Err(AppError::Refused(format!(
            "{label} isn't open: start it, then try again"
        )));
    };
    let what = match action {
        RouteAction::NewInFolder => "a new session",
        _ => "a new chat",
    };
    Ok(match desktop.send_link_to(pid, &link, false) {
        LinkSent::Sent => outcome(format!("Opened {what} in {label}"), "thread"),
        LinkSent::NeedsConsent => outcome(
            format!(
                "{label} needs your OK once for AI Juggler to send it a link: use Go to it to allow it"
            ),
            "consent",
        ),
        LinkSent::Denied => outcome(
            format!(
                "To open it for you, allow AI Juggler for {label} in System Settings › Privacy & Security › Automation"
            ),
            "none",
        ),
        LinkSent::Failed => outcome(format!("Couldn't open {what} in {label}"), "none"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn claude() -> HostApp {
        juggler_adapters::claude_live::host_apps().remove(0)
    }

    #[test]
    fn the_claude_app_opens_a_new_chat_and_a_folder_here_and_declares_the_rest() {
        let reach = info(&claude()).unwrap();
        assert_eq!(reach.seen, "installed");
        assert_eq!(reach.open_here, ["new_chat", "new_in_folder"]);
        let status = |action: &str| {
            reach
                .routes
                .iter()
                .find(|route| route.action == action)
                .map(|route| (route.status.as_str(), route.usable))
        };
        assert_eq!(status("new_chat"), Some(("documented", true)));
        assert_eq!(status("open_chat"), Some(("documented", true)));
        assert_eq!(status("new_in_folder"), Some(("documented", true)));
        assert_eq!(status("open_session"), Some(("live_checked", true)));
        assert_eq!(status("continue_session"), Some(("unverified", false)));
        assert_eq!(status("needs_input"), Some(("unverified", false)));
        assert_eq!(status("reply"), None);
        assert!(reach.track_only.iter().any(|track| track.part == "reply"));
    }

    #[test]
    fn every_desktop_app_says_what_it_allows_and_a_terminal_nothing() {
        let hosts = HostRegistry::new(Vec::new());
        for app in juggler_adapters::desktop_apps::all() {
            let reach = info(hosts.by_id(&app.id).unwrap()).unwrap();
            assert!(
                !reach.track_only.is_empty() || app.id == "hermes-app",
                "{}",
                app.id
            );
            for track in &reach.track_only {
                assert!(
                    !track.why.is_empty() && !track.part.is_empty(),
                    "{}",
                    app.id
                );
            }
        }
        assert_eq!(info(hosts.by_id("terminal").unwrap()), None);
        // Only the Claude app and Kimi Code open something here.
        let open: Vec<(String, Vec<String>)> = juggler_adapters::desktop_apps::all()
            .iter()
            .filter_map(|app| info(app).map(|reach| (app.id.clone(), reach.open_here)))
            .filter(|(_, open)| !open.is_empty())
            .collect();
        assert_eq!(
            open,
            [
                (
                    "claude-app".to_owned(),
                    vec!["new_chat".to_owned(), "new_in_folder".to_owned()]
                ),
                ("kimi-code-app".to_owned(), vec!["new_in_folder".to_owned()]),
            ]
        );
    }

    #[test]
    fn open_here_sends_only_a_new_chat_or_a_folder() {
        assert_eq!(open_here_action("new_chat"), Some(RouteAction::NewChat));
        assert_eq!(
            open_here_action("new_in_folder"),
            Some(RouteAction::NewInFolder)
        );
        for word in [
            "continue_session",
            "needs_input",
            "open_session",
            "reply",
            "nope",
            "",
        ] {
            assert_eq!(open_here_action(word), None, "{word}");
        }
    }
}
