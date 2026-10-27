//! AJ-E2501 (§4.17) — the menu-bar item: the same count as the bell and
//! the badge beside its icon, and a menu of what needs the user and what
//! is open now, with Open AI Juggler and New session…. A line opens its
//! notice in AI Juggler; nothing here acts on a session by itself.
//!
//! [`build`] is plain data (tested); [`apply`] draws it with Tauri's
//! tray API, the same on every desktop system (Windows and Linux show the
//! count in the tooltip where a title can't be drawn).

use super::centre::NoticeInfo;
use super::views::OpenNowInput;

/// Lines of Needs you the menu lists before "and N more…".
pub const NEEDS_LISTED: usize = 5;
/// The menu-bar item's id.
pub const TRAY_ID: &str = "aij-tray";
/// Menu ids: open AI Juggler, start a new session, open the centre.
pub const OPEN_ID: &str = "tray:open";
/// See [`OPEN_ID`].
pub const NEW_SESSION_ID: &str = "tray:new-session";
/// See [`OPEN_ID`].
pub const CENTRE_ID: &str = "tray:centre";
/// A notice's line: `tray:notice:<id>`.
pub const NOTICE_PREFIX: &str = "tray:notice:";

/// One line of the menu.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TrayLine {
    /// A section's name (not clickable).
    Heading {
        /// Its words.
        text: String,
    },
    /// A line that opens something in AI Juggler.
    Item {
        /// The menu id ([`OPEN_ID`], [`NOTICE_PREFIX`]…).
        id: String,
        /// Its words.
        text: String,
    },
    /// A line that only says something.
    Note {
        /// Its words.
        text: String,
    },
    /// Between groups.
    Separator,
}

/// The whole menu-bar item.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct TrayMenu {
    /// Beside the icon: the count, when anything needs the user.
    pub title: Option<String>,
    /// On hover.
    pub tooltip: String,
    /// The menu.
    pub lines: Vec<TrayLine>,
}

/// How long it has waited, in a few characters ("24 m", "3 h").
fn waited(created_ms: i64, now: i64) -> String {
    let minutes = ((now - created_ms).max(0)) / 60_000;
    if minutes < 1 {
        "now".to_owned()
    } else if minutes < 60 {
        format!("{minutes} m")
    } else {
        format!("{} h", minutes / 60)
    }
}

fn line_of(notice: &NoticeInfo, now: i64) -> String {
    let title: String = notice.title.chars().take(56).collect();
    let title = if notice.title.chars().count() > 56 {
        format!("{}…", title.trim_end())
    } else {
        title
    };
    let meta = match notice.kind.as_str() {
        "quota" => notice.meter.map(|left| format!("{left}%")),
        "asks" | "stuck" | "message" | "task" => Some(waited(notice.created_ms, now)),
        _ => None,
    };
    meta.map_or_else(|| title.clone(), |meta| format!("{title} · {meta}"))
}

/// The menu for `count` needs (the first of them in `needs`) and what is
/// open now.
#[must_use]
pub fn build(count: u32, needs: &[NoticeInfo], open: &OpenNowInput, now: i64) -> TrayMenu {
    let mut lines = vec![TrayLine::Heading {
        text: format!("Needs you · {count}"),
    }];
    if needs.is_empty() {
        lines.push(TrayLine::Note {
            text: "Nothing needs you".to_owned(),
        });
    }
    for notice in needs.iter().take(NEEDS_LISTED) {
        lines.push(TrayLine::Item {
            id: format!("{NOTICE_PREFIX}{}", notice.id),
            text: line_of(notice, now),
        });
    }
    let listed = u32::try_from(needs.len().min(NEEDS_LISTED)).unwrap_or(0);
    if count > listed {
        lines.push(TrayLine::Item {
            id: CENTRE_ID.to_owned(),
            text: format!("and {} more…", count - listed),
        });
    }
    lines.push(TrayLine::Separator);
    let working = if open.working > 0 {
        format!(" · {} working", open.working)
    } else {
        String::new()
    };
    lines.push(TrayLine::Heading {
        text: format!("Open now · {}{working}", open.total),
    });
    let apps: Vec<String> = open
        .apps
        .iter()
        .filter(|app| app.count > 0)
        .map(|app| format!("{} · {}", app.app, app.count))
        .collect();
    lines.push(TrayLine::Note {
        text: if apps.is_empty() {
            "Nothing open".to_owned()
        } else {
            apps.join(" · ")
        },
    });
    lines.push(TrayLine::Separator);
    lines.push(TrayLine::Item {
        id: OPEN_ID.to_owned(),
        text: "Open AI Juggler".to_owned(),
    });
    lines.push(TrayLine::Item {
        id: NEW_SESSION_ID.to_owned(),
        text: "New session…".to_owned(),
    });
    TrayMenu {
        title: (count > 0).then(|| count.to_string()),
        tooltip: match count {
            0 => "AI Juggler · nothing needs you".to_owned(),
            1 => "AI Juggler · 1 thing needs you".to_owned(),
            n => format!("AI Juggler · {n} things need you"),
        },
        lines,
    }
}

/// What a click on a menu id asks for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TrayClick {
    /// Bring AI Juggler forward.
    Open,
    /// Bring it forward on New session.
    NewSession,
    /// Bring it forward on the notification centre.
    Centre,
    /// Bring it forward on this notice's subject.
    Notice(i64),
}

/// The click a menu id stands for.
#[must_use]
pub fn click_of(id: &str) -> Option<TrayClick> {
    match id {
        OPEN_ID => Some(TrayClick::Open),
        NEW_SESSION_ID => Some(TrayClick::NewSession),
        CENTRE_ID => Some(TrayClick::Centre),
        _ => id
            .strip_prefix(NOTICE_PREFIX)
            .and_then(|id| id.parse().ok())
            .map(TrayClick::Notice),
    }
}

/// Draws `menu` on the menu-bar item (made the first time).
///
/// # Errors
///
/// Tauri's error when the menu or the item can't be made.
pub fn apply<R: tauri::Runtime>(app: &tauri::AppHandle<R>, menu: &TrayMenu) -> tauri::Result<()> {
    use tauri::menu::{IsMenuItem, MenuBuilder, MenuItem, PredefinedMenuItem};
    let mut items: Vec<Box<dyn IsMenuItem<R>>> = Vec::with_capacity(menu.lines.len());
    for (index, line) in menu.lines.iter().enumerate() {
        match line {
            TrayLine::Heading { text } | TrayLine::Note { text } => {
                items.push(Box::new(MenuItem::with_id(
                    app,
                    format!("tray:line:{index}"),
                    text,
                    false,
                    None::<&str>,
                )?));
            }
            TrayLine::Item { id, text } => {
                items.push(Box::new(MenuItem::with_id(
                    app,
                    id,
                    text,
                    true,
                    None::<&str>,
                )?));
            }
            TrayLine::Separator => items.push(Box::new(PredefinedMenuItem::separator(app)?)),
        }
    }
    let refs: Vec<&dyn IsMenuItem<R>> = items.iter().map(AsRef::as_ref).collect();
    let built = MenuBuilder::new(app).items(&refs).build()?;
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        tray.set_menu(Some(built))?;
        tray.set_title(menu.title.as_deref())?;
        tray.set_tooltip(Some(&menu.tooltip))?;
        return Ok(());
    }
    let mut builder = tauri::tray::TrayIconBuilder::with_id(TRAY_ID)
        .menu(&built)
        .tooltip(&menu.tooltip)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| super::views::tray_clicked(app, event.id().as_ref()));
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    if let Some(title) = &menu.title {
        builder = builder.title(title);
    }
    builder.build(app)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::notifications::centre::{NoticeFacts, NoticeTarget};
    use crate::features::notifications::views::OpenApp;

    const NOW: i64 = 1_800_000_000_000;

    fn need(id: i64, kind: &str, title: &str, minutes_ago: i64, meter: Option<i64>) -> NoticeInfo {
        NoticeInfo {
            id,
            kind: kind.to_owned(),
            needs: true,
            title: title.to_owned(),
            detail: String::new(),
            project_id: None,
            project: None,
            target: NoticeTarget::default(),
            facts: NoticeFacts::default(),
            action: None,
            meter,
            created_ms: NOW - minutes_ago * 60_000,
            unread: true,
            subject: String::new(),
            snoozed: false,
            snoozed_until_ms: None,
            muted: false,
            dismissed: false,
        }
    }

    fn open() -> OpenNowInput {
        OpenNowInput {
            total: 10,
            working: 4,
            apps: vec![
                OpenApp {
                    app: "Claude app".to_owned(),
                    count: 6,
                },
                OpenApp {
                    app: "Codex in ChatGPT".to_owned(),
                    count: 3,
                },
                OpenApp {
                    app: "Terminal".to_owned(),
                    count: 1,
                },
            ],
        }
    }

    fn texts(menu: &TrayMenu) -> Vec<String> {
        menu.lines
            .iter()
            .map(|line| match line {
                TrayLine::Heading { text } => format!("# {text}"),
                TrayLine::Item { id, text } => format!("[{id}] {text}"),
                TrayLine::Note { text } => format!("({text})"),
                TrayLine::Separator => "---".to_owned(),
            })
            .collect()
    }

    #[test]
    fn the_menu_lists_what_needs_you_then_what_is_open() {
        let needs = [
            need(1, "asks", "“Docs bot” needs your reply", 24, None),
            need(2, "quota", "Work Max is almost out of room", 3, Some(7)),
            need(
                3,
                "review",
                "“Site deployment” is done — review it",
                12,
                None,
            ),
        ];
        let menu = build(3, &needs, &open(), NOW);
        assert_eq!(menu.title.as_deref(), Some("3"));
        assert_eq!(menu.tooltip, "AI Juggler · 3 things need you");
        assert_eq!(
            texts(&menu),
            [
                "# Needs you · 3",
                "[tray:notice:1] “Docs bot” needs your reply · 24 m",
                "[tray:notice:2] Work Max is almost out of room · 7%",
                "[tray:notice:3] “Site deployment” is done — review it",
                "---",
                "# Open now · 10 · 4 working",
                "(Claude app · 6 · Codex in ChatGPT · 3 · Terminal · 1)",
                "---",
                "[tray:open] Open AI Juggler",
                "[tray:new-session] New session…",
            ]
        );
    }

    /// The gallery draws the menu from this builder's own output (the
    /// approved design's example): `AJ_WRITE_FIXTURES=1` writes it again.
    #[test]
    fn the_gallerys_menu_is_the_builders() {
        let needs = [
            need(1, "asks", "“Docs bot” needs your reply", 24, None),
            need(2, "quota", "Work Max is almost out of room", 3, Some(7)),
            need(
                3,
                "review",
                "“Site deployment” is done — review it",
                12,
                None,
            ),
        ];
        let menu = build(3, &needs, &open(), NOW);
        let json = serde_json::to_string_pretty(&menu).unwrap() + "\n";
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../ui/tests/visual/fixtures/tray-menu.json");
        if std::env::var_os("AJ_WRITE_FIXTURES").is_some() {
            std::fs::write(&path, &json).unwrap();
        }
        let kept = std::fs::read_to_string(&path).unwrap_or_default();
        assert!(
            kept == json,
            "the gallery's tray-menu.json is stale: run with AJ_WRITE_FIXTURES=1"
        );
    }

    #[test]
    fn nothing_needs_you_and_nothing_open_say_so_with_no_count() {
        let menu = build(0, &[], &OpenNowInput::default(), NOW);
        assert_eq!(menu.title, None);
        assert_eq!(menu.tooltip, "AI Juggler · nothing needs you");
        let lines = texts(&menu);
        assert!(lines.contains(&"(Nothing needs you)".to_owned()));
        assert!(lines.contains(&"(Nothing open)".to_owned()));
    }

    #[test]
    fn a_long_list_ends_with_the_rest_in_the_centre() {
        let needs: Vec<NoticeInfo> = (0..8)
            .map(|id| need(id, "review", "Done", 1, None))
            .collect();
        let menu = build(8, &needs, &OpenNowInput::default(), NOW);
        let lines = texts(&menu);
        assert_eq!(
            lines
                .iter()
                .filter(|line| line.starts_with("[tray:notice:"))
                .count(),
            NEEDS_LISTED
        );
        assert!(lines.contains(&"[tray:centre] and 3 more…".to_owned()));
    }

    #[test]
    fn every_id_the_menu_makes_is_a_click_it_knows() {
        let needs = [need(42, "asks", "A", 1, None)];
        let menu = build(9, &needs, &open(), NOW);
        for line in &menu.lines {
            if let TrayLine::Item { id, .. } = line {
                assert!(click_of(id).is_some(), "a menu line without a click");
            }
        }
        assert_eq!(click_of("tray:notice:42"), Some(TrayClick::Notice(42)));
        assert_eq!(click_of("tray:line:0"), None);
        assert_eq!(click_of("tray:notice:x"), None);
    }

    #[test]
    fn a_long_title_is_cut_with_an_ellipsis() {
        let long = "x".repeat(80);
        let menu = build(
            1,
            &[need(1, "review", &long, 0, None)],
            &OpenNowInput::default(),
            NOW,
        );
        let TrayLine::Item { text, .. } = &menu.lines[1] else {
            unreachable!("the first line after the heading is the notice");
        };
        assert_eq!(text.chars().count(), 57);
        assert!(text.ends_with('…'));
    }
}
