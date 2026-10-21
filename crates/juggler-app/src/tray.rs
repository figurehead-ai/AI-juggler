//! AJ-E1411: This Mac in the menu bar — calm by design (D-25).
//!
//! One icon. A number beside it only when something needs a glance (an
//! overload alert, a session working the Mac hard). A click opens a menu
//! that *is* This Mac: the one thing most worth a glance first, the Mac's
//! CPU and memory, each AI tool's share, the heavy sessions, and Open This
//! Mac. Passive like the rest of E14: the menu opens the app, never stops
//! or starts anything.
//!
//! The model ([`tray_model`]) and the icon ([`icon_rgba`]) are pure and
//! tested on every platform; the native glue is macOS-only.

use crate::machine_alerts::OverloadAlertInfo;
use crate::machine_health::{ThisMacInfo, ToolGroupInfo, ToolTreeInfo};
use juggler_supervisor::machine::bytes_words;

/// What a menu line does when clicked.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TrayAction {
    /// Open the app on Home's This Mac.
    ThisMac,
    /// Open a session AI Juggler started.
    Session(String),
    /// Quit AI Juggler.
    Quit,
}

/// One line of the menu.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TrayItem {
    /// A line that does something.
    Action {
        /// The line in words.
        text: String,
        /// What a click does.
        action: TrayAction,
    },
    /// A figure, not clickable.
    Label(String),
    /// A divider.
    Separator,
}

impl TrayItem {
    /// The line's shape without its words: two menus with the same shape
    /// differ only in text, which is updated in place (an open menu stays
    /// open).
    #[must_use]
    pub fn shape(&self) -> (u8, Option<&TrayAction>) {
        match self {
            Self::Action { action, .. } => (0, Some(action)),
            Self::Label(_) => (1, None),
            Self::Separator => (2, None),
        }
    }

    /// The line's words (empty for a divider).
    #[must_use]
    pub fn text(&self) -> &str {
        match self {
            Self::Action { text, .. } | Self::Label(text) => text,
            Self::Separator => "",
        }
    }
}

/// The menu-bar item, as a value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrayModel {
    /// Beside the icon: how many things need a glance, only when some do.
    pub title: Option<String>,
    /// Whether the icon takes its attention shape (a dot beside it).
    pub attention: bool,
    /// The icon's tooltip.
    pub tooltip: String,
    /// The menu, top to bottom.
    pub items: Vec<TrayItem>,
}

/// A long session name is cut to this many characters in the menu.
const NAME_CHARS: usize = 40;

fn short(name: &str) -> String {
    if name.chars().count() <= NAME_CHARS {
        return name.to_string();
    }
    let cut: String = name.chars().take(NAME_CHARS - 1).collect();
    format!("{cut}…")
}

/// CPU as the menu says it ("CPU 34%"; "CPU unknown" when unmeasured —
/// words a screen reader reads, not a dash).
fn cpu_words(cpu: Option<f64>) -> String {
    cpu.map_or_else(|| "CPU unknown".to_string(), |cpu| format!("CPU {cpu:.0}%"))
}

/// Memory as the menu says it: "about" when the sum is an estimate.
fn memory_words(bytes: u64, estimate: bool) -> String {
    if estimate {
        format!("about {}", bytes_words(bytes))
    } else {
        bytes_words(bytes)
    }
}

/// "2 sessions", "3 windows", "1 running".
fn count_words(group: &ToolGroupInfo) -> String {
    let n = group.totals.count;
    let (one, many) = match (group.ours, group.tool.as_str()) {
        (true, _) => ("session", "sessions"),
        (false, "desktop") => ("window", "windows"),
        _ => ("running", "running"),
    };
    format!("{n} {}", if n == 1 { one } else { many })
}

/// Who an alert is about, and what of theirs is high — as Home says it.
fn alert_subject(
    alert: &OverloadAlertInfo,
    name_of: &dyn Fn(&str) -> Option<String>,
) -> Option<String> {
    let who = match (&alert.session_id, alert.place.as_deref()) {
        (Some(id), _) => short(&name_of(id).unwrap_or_else(|| "a session".into())),
        (None, Some("desktop")) => "The Claude app".into(),
        (None, Some(_)) => "claude in a terminal".into(),
        (None, None) => return None,
    };
    let mut figures = Vec::new();
    if let Some(cpu) = alert.subject_cpu {
        figures.push(format!("CPU {cpu:.0}%"));
    }
    if let Some(bytes) = alert.subject_bytes {
        figures.push(memory_words(bytes, alert.subject_estimate));
    }
    (!figures.is_empty()).then(|| format!("{who} is using {}", figures.join(" and ")))
}

/// The order alerts are worth a glance in: the Mac itself first.
fn alert_rank(kind: &str) -> u8 {
    match kind {
        "memory_pressure" => 0,
        "swap" => 1,
        "thermal" => 2,
        "disk" => 3,
        "cpu" => 4,
        _ => 5,
    }
}

/// Our heavy sessions (a tool AI Juggler didn't start is This Mac's to
/// show, never a Needs-you item — D-25 amendment).
fn heavy_ours(info: &ThisMacInfo) -> impl Iterator<Item = (&str, &ToolTreeInfo)> {
    info.heavy
        .iter()
        .filter_map(|tree| tree.session_id.as_deref().map(|id| (id, tree)))
}

/// How many things need a glance: each alert, and each heavy session of
/// ours (a heavy session's own alert counts once).
fn needs_glance(info: &ThisMacInfo) -> usize {
    let mut keys: std::collections::BTreeSet<String> =
        info.alerts.iter().map(|alert| alert.key.clone()).collect();
    keys.extend(heavy_ours(info).map(|(id, _)| format!("heavy:{id}")));
    keys.len()
}

/// The one line most worth a glance, and where a click takes you.
fn glance(info: &ThisMacInfo, name_of: &dyn Fn(&str) -> Option<String>) -> (String, TrayAction) {
    if let Some(alert) = info
        .alerts
        .iter()
        .min_by_key(|alert| alert_rank(&alert.kind))
    {
        let text = match alert_subject(alert, name_of) {
            Some(subject) => format!("{} — {subject}", alert.title),
            None => format!("{} — {}", alert.title, alert.detail),
        };
        let action = alert
            .session_id
            .clone()
            .map_or(TrayAction::ThisMac, TrayAction::Session);
        return (text, action);
    }
    if let Some((id, tree)) = heavy_ours(info).next() {
        let name = short(&name_of(id).unwrap_or_else(|| "A session".into()));
        let note = tree.heavy_note.as_deref().unwrap_or_default();
        return (
            format!("{name} is working your Mac hard — {note}"),
            TrayAction::Session(id.to_string()),
        );
    }
    let summary = &info.machine.summary;
    if summary.level != "plenty" {
        let text = summary.reasons.first().map_or_else(
            || summary.words.clone(),
            |reason| format!("{} — {}", summary.words, reason.words),
        );
        return (text, TrayAction::ThisMac);
    }
    let text = if info.tools.count == 0 {
        "Nothing running — your Mac has room".to_string()
    } else {
        summary.words.clone()
    };
    (text, TrayAction::ThisMac)
}

/// The Mac's own figures in one line.
fn machine_line(info: &ThisMacInfo) -> String {
    let machine = &info.machine;
    if !machine.sampled {
        return "Measuring this Mac…".to_string();
    }
    let memory = match machine.memory_used {
        Some(used) => format!(
            "Memory {} of {}",
            bytes_words(used),
            bytes_words(machine.memory_total)
        ),
        None => format!("Memory {}", bytes_words(machine.memory_total)),
    };
    format!("{} · {memory}", cpu_words(machine.cpu_pct))
}

/// Pure: the menu-bar item for This Mac now. `name_of` names a session of
/// ours as the rail does (its branch, else its folder).
#[must_use]
pub fn tray_model(info: &ThisMacInfo, name_of: &dyn Fn(&str) -> Option<String>) -> TrayModel {
    let needs = needs_glance(info);
    let (first, first_action) = glance(info, name_of);
    let mut items = vec![
        TrayItem::Action {
            text: first.clone(),
            action: first_action.clone(),
        },
        TrayItem::Label(machine_line(info)),
        TrayItem::Separator,
    ];
    if info.by_tool.is_empty() {
        items.push(TrayItem::Label("No AI tools running".into()));
    }
    for group in &info.by_tool {
        items.push(TrayItem::Label(format!(
            "{} · {} · {} · {}",
            group.label,
            count_words(group),
            cpu_words(group.totals.cpu_pct),
            memory_words(group.totals.rss_bytes, group.totals.memory_estimate),
        )));
    }
    let heavy: Vec<TrayItem> = heavy_ours(info)
        .filter(|(id, _)| first_action != TrayAction::Session((*id).to_string()))
        .map(|(id, tree)| TrayItem::Action {
            text: format!(
                "{} — {}",
                short(&name_of(id).unwrap_or_else(|| "A session".into())),
                tree.heavy_note.as_deref().unwrap_or_default()
            ),
            action: TrayAction::Session(id.to_string()),
        })
        .collect();
    if !heavy.is_empty() {
        items.push(TrayItem::Separator);
        items.push(TrayItem::Label("Working your Mac hard".into()));
        items.extend(heavy);
    }
    items.push(TrayItem::Separator);
    items.push(TrayItem::Action {
        text: "Open This Mac".into(),
        action: TrayAction::ThisMac,
    });
    items.push(TrayItem::Action {
        text: "Quit AI Juggler".into(),
        action: TrayAction::Quit,
    });
    TrayModel {
        title: (needs > 0).then(|| needs.to_string()),
        attention: needs > 0 || info.machine.summary.level == "pressure",
        // The number beside the icon is only a digit; the tooltip (which
        // a screen reader reads as the item's help) says what it counts.
        tooltip: match needs {
            0 => format!("AI Juggler — {first}"),
            1 => format!("AI Juggler — 1 needs a glance: {first}"),
            n => format!("AI Juggler — {n} need a glance: {first}"),
        },
        items,
    }
}

/// Pure: the menu bar while This Mac can't be read (two ticks in a row):
/// old numbers never stay up as if they were current.
#[must_use]
pub fn unmeasured_model() -> TrayModel {
    TrayModel {
        title: None,
        attention: false,
        tooltip: "AI Juggler — this Mac isn't measured right now".into(),
        items: vec![
            TrayItem::Label("Not measured right now".into()),
            TrayItem::Separator,
            TrayItem::Action {
                text: "Open This Mac".into(),
                action: TrayAction::ThisMac,
            },
            TrayItem::Action {
                text: "Quit AI Juggler".into(),
                action: TrayAction::Quit,
            },
        ],
    }
}

/// How many This Mac readings may fail in a row before the menu bar says
/// it isn't measured (one miss is a blip; the next would be 10 s stale).
pub const MISSES_BEFORE_UNMEASURED: u32 = 2;

fn action_key(action: &TrayAction) -> String {
    match action {
        TrayAction::ThisMac => "this-mac".into(),
        TrayAction::Quit => "quit".into(),
        TrayAction::Session(id) => format!("session:{id}"),
    }
}

/// Pure: a menu line's id — a serial (unique per line ever made) and what
/// the line does. A click resolves by the action in its own id, so a menu
/// that changed while it was open can never turn a click on one line into
/// another line's action.
#[must_use]
pub fn item_id(serial: u64, item: &TrayItem) -> String {
    match item {
        TrayItem::Action { action, .. } => format!("tray:{serial}:{}", action_key(action)),
        TrayItem::Label(_) => format!("tray:{serial}:label"),
        TrayItem::Separator => format!("tray:{serial}:divider"),
    }
}

/// Pure: what a click on the line with this id does; `None` (nothing) for
/// a figure, a divider or an id that isn't ours.
#[must_use]
pub fn action_of(id: &str) -> Option<TrayAction> {
    let (_, key) = id.strip_prefix("tray:")?.split_once(':')?;
    match key {
        "this-mac" => Some(TrayAction::ThisMac),
        "quit" => Some(TrayAction::Quit),
        _ => key
            .strip_prefix("session:")
            .filter(|session| !session.is_empty())
            .map(|session| TrayAction::Session(session.to_string())),
    }
}

/// How to turn the menu on screen into the new one without replacing it
/// (replacing it would close it while open): the lines that keep their
/// place get new words, and only the changed run in between is removed
/// and inserted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuPlan {
    /// Lines that keep their place: (position in the old menu, new words).
    pub retext: Vec<(usize, String)>,
    /// Old lines to remove (positions in the old menu).
    pub remove: std::ops::Range<usize>,
    /// New lines to insert where the removed ones were (positions in the
    /// new model).
    pub insert: std::ops::Range<usize>,
}

/// Pure: the plan from `old` to `new` — the shared start and end (by
/// shape) stay, the run between is replaced.
#[must_use]
pub fn plan(old: &[TrayItem], new: &[TrayItem]) -> MenuPlan {
    let prefix = old
        .iter()
        .zip(new)
        .take_while(|(a, b)| a.shape() == b.shape())
        .count();
    let room = old.len().min(new.len()) - prefix;
    let suffix = old
        .iter()
        .rev()
        .zip(new.iter().rev())
        .take(room)
        .take_while(|(a, b)| a.shape() == b.shape())
        .count();
    let kept = (0..prefix)
        .map(|i| (i, i))
        .chain((0..suffix).map(|k| (old.len() - suffix + k, new.len() - suffix + k)));
    let retext = kept
        .filter(|&(o, n)| old[o].text() != new[n].text())
        .map(|(o, n)| (o, new[n].text().to_string()))
        .collect();
    MenuPlan {
        retext,
        remove: prefix..old.len() - suffix,
        insert: prefix..new.len() - suffix,
    }
}

/// The icon's side, pixels (18 points at 2×).
pub const ICON_PX: u32 = 36;

/// Pure: the menu-bar icon as a template image (black, alpha only — macOS
/// tints it for a light or dark menu bar). Three balls in an arc; the
/// attention shape adds a dot at the top right, so the state is a shape,
/// never colour alone.
#[must_use]
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]
pub fn icon_rgba(attention: bool) -> Vec<u8> {
    // (centre x, centre y, radius) in pixels.
    let mut discs: Vec<(f32, f32, f32)> =
        vec![(8.0, 25.0, 4.5), (18.0, 13.0, 4.5), (28.0, 25.0, 4.5)];
    if attention {
        discs.push((30.0, 6.0, 4.0));
    }
    let mut rgba = vec![0_u8; (ICON_PX * ICON_PX * 4) as usize];
    for y in 0..ICON_PX {
        for x in 0..ICON_PX {
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            // Coverage with a one-pixel soft edge (anti-aliased).
            let coverage = discs
                .iter()
                .map(|&(cx, cy, r)| {
                    let distance = ((px - cx).powi(2) + (py - cy).powi(2)).sqrt();
                    (r + 0.5 - distance).clamp(0.0, 1.0)
                })
                .fold(0.0_f32, f32::max);
            let alpha = (coverage * 255.0).round() as u8;
            let at = ((y * ICON_PX + x) * 4) as usize;
            rgba[at + 3] = alpha;
        }
    }
    rgba
}

#[cfg(target_os = "macos")]
pub use native::{install, set_shown, unmeasured, update};

/// The app setting that hides the menu-bar item (`off`); anything else,
/// or never set, shows it.
pub const SETTING_KEY: &str = "menu_bar";

/// Whether the menu-bar item shows (Settings › Appearance). `None` where
/// this platform has none.
///
/// # Errors
///
/// Storage failures.
#[tauri::command]
pub async fn menu_bar_get(
    state: tauri::State<'_, std::sync::Arc<crate::state::AppState>>,
) -> crate::error::AppResult<Option<bool>> {
    if !cfg!(target_os = "macos") {
        return Ok(None);
    }
    let storage = state
        .storage
        .lock()
        .map_err(|_| crate::error::AppError::StatePoisoned)?;
    let value = storage.app_settings().get(SETTING_KEY)?;
    Ok(Some(value.as_deref() != Some("off")))
}

/// Shows or hides the menu-bar item, now and at the next launch. Async, so
/// it never runs on the main thread the status item's calls hop to.
///
/// # Errors
///
/// Storage failures.
#[tauri::command]
pub async fn menu_bar_set(
    app: tauri::AppHandle,
    state: tauri::State<'_, std::sync::Arc<crate::state::AppState>>,
    shown: bool,
) -> crate::error::AppResult<()> {
    {
        let storage = state
            .storage
            .lock()
            .map_err(|_| crate::error::AppError::StatePoisoned)?;
        storage
            .app_settings()
            .set(SETTING_KEY, if shown { "on" } else { "off" })?;
    }
    #[cfg(target_os = "macos")]
    set_shown(&app, shown);
    #[cfg(not(target_os = "macos"))]
    let _ = app;
    Ok(())
}

#[cfg(target_os = "macos")]
mod native {
    use std::sync::{Arc, Mutex};

    use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
    use tauri::tray::{TrayIcon, TrayIconBuilder};
    use tauri::{AppHandle, Emitter, Manager, Wry};

    use super::{
        ICON_PX, SETTING_KEY, TrayAction, TrayItem, TrayModel, action_of, icon_rgba, item_id, plan,
        tray_model, unmeasured_model,
    };
    use crate::machine_health::ThisMacInfo;
    use crate::state::AppState;

    /// What a tray click asks the window to open (`tray://open`).
    pub const EVENT_TRAY_OPEN: &str = "tray://open";

    /// One line on screen.
    #[derive(Clone)]
    enum Entry {
        Line(MenuItem<Wry>),
        Divider(PredefinedMenuItem<Wry>),
    }

    /// The live tray. Its lock guards these handles and the model only;
    /// it is never held across a call into the status item or its menu
    /// (each waits for the main thread, where a click runs).
    struct Tray {
        icon: TrayIcon,
        menu: Menu<Wry>,
        model: TrayModel,
        entries: Vec<Entry>,
        serial: u64,
    }

    struct TrayState(Mutex<Option<Tray>>);

    fn image(attention: bool) -> tauri::image::Image<'static> {
        tauri::image::Image::new_owned(icon_rgba(attention), ICON_PX, ICON_PX)
    }

    fn make(app: &AppHandle, serial: u64, item: &TrayItem) -> tauri::Result<Entry> {
        Ok(match item {
            TrayItem::Separator => Entry::Divider(PredefinedMenuItem::separator(app)?),
            TrayItem::Action { text, .. } | TrayItem::Label(text) => {
                Entry::Line(MenuItem::with_id(
                    app,
                    item_id(serial, item),
                    text,
                    matches!(item, TrayItem::Action { .. }),
                    None::<&str>,
                )?)
            }
        })
    }

    fn open_window(app: &AppHandle) {
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.show();
            let _ = window.unminimize();
            let _ = window.set_focus();
        }
    }

    /// A click: resolved from the line's own id — no lock, no lookup.
    fn on_click(app: &AppHandle, id: &str) {
        match action_of(id) {
            Some(TrayAction::Quit) => app.exit(0),
            Some(TrayAction::ThisMac) => {
                open_window(app);
                let _ = app.emit(EVENT_TRAY_OPEN, serde_json::json!({ "target": "this-mac" }));
            }
            Some(TrayAction::Session(session)) => {
                open_window(app);
                let _ = app.emit(
                    EVENT_TRAY_OPEN,
                    serde_json::json!({ "target": "session", "session_id": session }),
                );
            }
            None => {}
        }
    }

    /// Puts the icon in the menu bar (once, at startup), unless the user
    /// turned it off. Until the first reading it says "Measuring this Mac…".
    ///
    /// # Errors
    ///
    /// When macOS refuses the status item or its menu.
    pub fn install(app: &AppHandle, state: &Arc<AppState>) -> tauri::Result<()> {
        let mut first = unmeasured_model();
        first.items[0] = TrayItem::Label("Measuring this Mac…".into());
        first.tooltip = "AI Juggler".into();
        let menu = Menu::new(app)?;
        let mut entries = Vec::with_capacity(first.items.len());
        for (serial, item) in (0_u64..).zip(&first.items) {
            let entry = make(app, serial, item)?;
            match &entry {
                Entry::Line(line) => menu.append(line)?,
                Entry::Divider(divider) => menu.append(divider)?,
            }
            entries.push(entry);
        }
        let icon = TrayIconBuilder::with_id("this-mac")
            .icon(image(false))
            .icon_as_template(true)
            .tooltip(&first.tooltip)
            .menu(&menu)
            .show_menu_on_left_click(true)
            .on_menu_event(|app, event| on_click(app, event.id().as_ref()))
            .build(app)?;
        let hidden = state
            .storage
            .lock()
            .ok()
            .and_then(|storage| storage.app_settings().get(SETTING_KEY).ok().flatten());
        if hidden.as_deref() == Some("off") {
            icon.set_visible(false)?;
        }
        app.manage(TrayState(Mutex::new(Some(Tray {
            icon,
            menu,
            serial: u64::try_from(entries.len()).unwrap_or(0),
            model: first,
            entries,
        }))));
        Ok(())
    }

    /// Shows or hides the item now (Settings › Appearance).
    pub fn set_shown(app: &AppHandle, shown: bool) {
        let icon = app
            .try_state::<TrayState>()
            .and_then(|state| state.0.lock().ok()?.as_ref().map(|tray| tray.icon.clone()));
        if let Some(icon) = icon {
            let _ = icon.set_visible(shown);
        }
    }

    /// The rail's name for a session of ours: its branch, else its folder.
    fn name_of(state: &Arc<AppState>, id: &str) -> Option<String> {
        let storage = state.storage.lock().ok()?;
        let record = storage
            .sessions()
            .get(&juggler_core::model::SessionId::new(id))
            .ok()?;
        crate::worktree::branch_of(&record.cwd).or_else(|| {
            record
                .cwd
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
        })
    }

    /// Shows This Mac now (the 5 s cadence).
    pub fn update(app: &AppHandle, state: &Arc<AppState>, info: &ThisMacInfo) {
        show(app, tray_model(info, &|id| name_of(state, id)));
    }

    /// Says This Mac isn't measured right now (readings keep failing).
    pub fn unmeasured(app: &AppHandle) {
        show(app, unmeasured_model());
    }

    /// Moves the menu bar to `model`: nothing changes on screen unless the
    /// words do; lines are retexted, removed and inserted in place, never
    /// the whole menu replaced (an open menu stays open). Only the 5 s
    /// thread calls this, so the handles taken under the lock stay current
    /// while the native calls run with the lock released.
    fn show(app: &AppHandle, model: TrayModel) {
        let Some(tray_state) = app.try_state::<TrayState>() else {
            return;
        };
        let (icon, menu, last, mut entries, mut serial) = {
            let Ok(guard) = tray_state.0.lock() else {
                return;
            };
            let Some(tray) = guard.as_ref() else {
                return;
            };
            if tray.model == model {
                return;
            }
            (
                tray.icon.clone(),
                tray.menu.clone(),
                tray.model.clone(),
                tray.entries.clone(),
                tray.serial,
            )
        };
        // Debug builds can print what the menu says (`AIJ_TRAY_TRACE=1`):
        // the status item can't be read back without Screen Recording.
        if cfg!(debug_assertions) && std::env::var_os("AIJ_TRAY_TRACE").is_some() {
            let lines: Vec<&str> = model.items.iter().map(TrayItem::text).collect();
            eprintln!(
                "[tray] title={:?} attention={} tooltip={:?} | {}",
                model.title,
                model.attention,
                model.tooltip,
                lines.join(" | ")
            );
        }
        if last.title != model.title {
            let _ = icon.set_title(model.title.as_deref());
        }
        if last.attention != model.attention {
            let _ = icon.set_icon(Some(image(model.attention)));
            let _ = icon.set_icon_as_template(true);
        }
        if last.tooltip != model.tooltip {
            let _ = icon.set_tooltip(Some(&model.tooltip));
        }
        let steps = plan(&last.items, &model.items);
        for (position, words) in &steps.retext {
            if let Some(Entry::Line(line)) = entries.get(*position) {
                let _ = line.set_text(words);
            }
        }
        for _ in steps.remove.clone() {
            let _ = menu.remove_at(steps.remove.start);
        }
        let mut inserted = Vec::new();
        for (offset, item) in model.items[steps.insert.clone()].iter().enumerate() {
            let Ok(entry) = make(app, serial, item) else {
                continue;
            };
            serial += 1;
            let at = steps.insert.start + offset;
            let _ = match &entry {
                Entry::Line(line) => menu.insert(line, at),
                Entry::Divider(divider) => menu.insert(divider, at),
            };
            inserted.push(entry);
        }
        entries.splice(steps.remove.clone(), inserted);
        if let Ok(mut guard) = tray_state.0.lock()
            && let Some(tray) = guard.as_mut()
        {
            tray.model = model;
            tray.entries = entries;
            tray.serial = serial;
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use crate::machine_health::{
        MachineHealthInfo, RoomReasonInfo, RoomSummaryInfo, ToolTotalsInfo,
    };

    const G: u64 = 1_000_000_000;

    fn calm() -> ThisMacInfo {
        ThisMacInfo {
            machine: MachineHealthInfo {
                sampled: true,
                cores: 10,
                cpu_pct: Some(34.0),
                memory_total: 32 * G,
                memory_used: Some(21 * G),
                memory_basis: Some("app_wired_compressed".into()),
                memory_available: 11 * G,
                pressure: Some("normal".into()),
                swap_total: 0,
                swap_used: 0,
                swap_growth_mb_per_min: Some(0.0),
                volumes: Vec::new(),
                power: None,
                thermal: Some("nominal".into()),
                summary: RoomSummaryInfo {
                    level: "plenty".into(),
                    words: "Plenty of room".into(),
                    reasons: Vec::new(),
                },
                series: Vec::new(),
            },
            tools: ToolTotalsInfo {
                count: 4,
                memory_estimate: true,
                cpu_pct: Some(9.0),
                rss_bytes: 5_700_000_000,
            },
            ours: ToolTotalsInfo::default(),
            theirs: ToolTotalsInfo::default(),
            by_tool: vec![
                group("claude-code", "Claude Code", true, 2, 6.0, 4_100_000_000),
                group("desktop", "Claude app", false, 2, 3.0, 1_600_000_000),
            ],
            top: Vec::new(),
            heavy: Vec::new(),
            leftover_processes: 0,
            leftover_ports: 0,
            alerts: Vec::new(),
            recommendations: Vec::new(),
        }
    }

    fn group(tool: &str, label: &str, ours: bool, count: u32, cpu: f64, rss: u64) -> ToolGroupInfo {
        ToolGroupInfo {
            tool: tool.into(),
            label: label.into(),
            ours,
            totals: ToolTotalsInfo {
                count,
                memory_estimate: !ours,
                cpu_pct: Some(cpu),
                rss_bytes: rss,
            },
        }
    }

    fn heavy(id: &str, note: &str) -> ToolTreeInfo {
        ToolTreeInfo {
            session_id: Some(id.into()),
            place: None,
            account_label: None,
            cpu_pct: Some(96.0),
            rss_bytes: 3_200_000_000,
            memory_estimate: false,
            helpers: 4,
            heavy_note: Some(note.into()),
        }
    }

    fn names(id: &str) -> Option<String> {
        match id {
            "s-1" => Some("feat/read-model".into()),
            "s-2" => Some("payments-api".into()),
            _ => None,
        }
    }

    fn texts(model: &TrayModel) -> Vec<&str> {
        model.items.iter().map(TrayItem::text).collect()
    }

    #[test]
    fn a_calm_mac_is_one_quiet_icon_with_this_mac_behind_a_click() {
        let model = tray_model(&calm(), &names);
        assert_eq!(model.title, None, "no number while nothing needs a glance");
        assert!(!model.attention);
        assert_eq!(
            texts(&model),
            [
                "Plenty of room",
                "CPU 34% · Memory 21 GB of 32 GB",
                "",
                "Claude Code · 2 sessions · CPU 6% · 4.1 GB",
                "Claude app · 2 windows · CPU 3% · about 1.6 GB",
                "",
                "Open This Mac",
                "Quit AI Juggler",
            ]
        );
        assert_eq!(model.items[0].shape().1, Some(&TrayAction::ThisMac));
    }

    #[test]
    fn nothing_running_says_so() {
        let mut info = calm();
        info.tools.count = 0;
        info.by_tool.clear();
        let model = tray_model(&info, &names);
        assert_eq!(model.items[0].text(), "Nothing running — your Mac has room");
        assert!(texts(&model).contains(&"No AI tools running"));
    }

    #[test]
    fn a_heavy_session_puts_a_number_up_and_comes_first() {
        let mut info = calm();
        info.heavy = vec![heavy("s-1", "CPU 96%"), heavy("s-2", "3.2 GB")];
        let model = tray_model(&info, &names);
        assert_eq!(model.title.as_deref(), Some("2"));
        assert!(model.attention);
        assert_eq!(
            model.items[0],
            TrayItem::Action {
                text: "feat/read-model is working your Mac hard — CPU 96%".into(),
                action: TrayAction::Session("s-1".into()),
            }
        );
        // The other heavy one is listed once, under its own heading.
        let rest = texts(&model);
        assert!(rest.contains(&"Working your Mac hard"));
        assert!(rest.contains(&"payments-api — 3.2 GB"));
        assert!(
            !rest[1..]
                .iter()
                .any(|line| line.starts_with("feat/read-model"))
        );
    }

    #[test]
    fn the_mac_itself_outranks_a_session_and_a_heavy_alert_counts_once() {
        let mut info = calm();
        info.heavy = vec![heavy("s-1", "CPU 96%")];
        info.alerts = vec![
            alert(
                "heavy:s-1",
                "heavy",
                "A session is working your Mac hard",
                Some("s-1"),
                None,
            ),
            alert(
                "memory_pressure",
                "memory_pressure",
                "Your Mac is under memory pressure",
                None,
                Some("desktop"),
            ),
        ];
        info.alerts[1].subject_bytes = Some(12 * G);
        info.alerts[1].subject_estimate = true;
        info.alerts[1].subject_cpu = None; // a memory alert tells memory only
        let model = tray_model(&info, &names);
        assert_eq!(
            model.title.as_deref(),
            Some("2"),
            "the heavy session's alert counts once"
        );
        assert_eq!(
            model.items[0].text(),
            "Your Mac is under memory pressure — The Claude app is using about 12 GB"
        );
        assert_eq!(model.items[0].shape().1, Some(&TrayAction::ThisMac));
    }

    #[test]
    fn a_tight_mac_says_why_without_a_number() {
        let mut info = calm();
        info.machine.summary = RoomSummaryInfo {
            level: "tight".into(),
            words: "Getting tight".into(),
            reasons: vec![RoomReasonInfo {
                kind: "memory".into(),
                level: "tight".into(),
                words: "memory is getting tight".into(),
            }],
        };
        let model = tray_model(&info, &names);
        assert_eq!(model.title, None);
        assert!(!model.attention, "tight is calm; pressure is not");
        assert_eq!(
            model.items[0].text(),
            "Getting tight — memory is getting tight"
        );
        info.machine.summary.level = "pressure".into();
        assert!(tray_model(&info, &names).attention);
    }

    #[test]
    fn before_the_first_reading_it_says_so() {
        let mut info = calm();
        info.machine.sampled = false;
        assert_eq!(
            tray_model(&info, &names).items[1].text(),
            "Measuring this Mac…"
        );
    }

    #[test]
    fn a_long_name_is_cut() {
        let mut info = calm();
        info.heavy = vec![heavy("s-long", "CPU 96%")];
        let long = |_: &str| Some("x".repeat(60));
        let model = tray_model(&info, &long);
        assert!(
            model.items[0]
                .text()
                .starts_with(&format!("{}…", "x".repeat(39)))
        );
    }

    #[test]
    fn the_icon_is_a_template_and_attention_is_a_shape() {
        let calm = icon_rgba(false);
        let busy = icon_rgba(true);
        assert_eq!(calm.len(), (ICON_PX * ICON_PX * 4) as usize);
        // Black with alpha only: macOS tints a template for the menu bar.
        assert!(
            calm.chunks(4)
                .all(|px| px[0] == 0 && px[1] == 0 && px[2] == 0)
        );
        let ink = |rgba: &[u8]| rgba.chunks(4).filter(|px| px[3] > 0).count();
        assert!(ink(&calm) > 150, "three balls");
        assert!(ink(&busy) > ink(&calm), "the attention dot adds ink");
        // The dot sits top right, where the calm icon is empty.
        let at = |rgba: &[u8], x: u32, y: u32| rgba[((y * ICON_PX + x) * 4 + 3) as usize];
        assert_eq!(at(&calm, 30, 6), 0);
        assert_eq!(at(&busy, 30, 6), 255);
    }

    #[test]
    fn a_click_resolves_by_its_own_action_even_after_the_menu_changed() {
        let mut info = calm();
        info.heavy = vec![heavy("s-1", "CPU 96%"), heavy("s-2", "3.2 GB")];
        let open = tray_model(&info, &names);
        // The ids the open menu was built with.
        let ids: Vec<String> = (0_u64..)
            .zip(&open.items)
            .map(|(serial, item)| item_id(serial, item))
            .collect();
        let payments = open
            .items
            .iter()
            .position(|item| item.text() == "payments-api — 3.2 GB")
            .unwrap();
        // The heavy sessions calm down while the menu is open: the menu is
        // shorter, and Quit now sits where payments-api was.
        let calmer = tray_model(&calm(), &names);
        assert_ne!(
            calmer.items.get(payments).map(TrayItem::text),
            Some("payments-api — 3.2 GB"),
            "by position, that line is now something else"
        );
        // A click on the old line still means that session, never Quit.
        assert_eq!(
            action_of(&ids[payments]),
            Some(TrayAction::Session("s-2".into()))
        );
        assert_eq!(action_of(&ids[ids.len() - 1]), Some(TrayAction::Quit));
        assert_eq!(action_of(&ids[1]), None, "a figure does nothing");
        assert_eq!(action_of("tray:9:session:"), None);
        assert_eq!(action_of("something-else"), None, "not ours: a safe no-op");
    }

    /// Applies a plan to the old lines, as the native glue does.
    fn apply(old: &[TrayItem], new: &[TrayItem], steps: &MenuPlan) -> Vec<TrayItem> {
        let mut lines = old.to_vec();
        for (position, words) in &steps.retext {
            lines[*position] = match &lines[*position] {
                TrayItem::Action { action, .. } => TrayItem::Action {
                    text: words.clone(),
                    action: action.clone(),
                },
                TrayItem::Label(_) => TrayItem::Label(words.clone()),
                TrayItem::Separator => TrayItem::Separator,
            };
        }
        lines.splice(
            steps.remove.clone(),
            new[steps.insert.clone()].iter().cloned(),
        );
        lines
    }

    #[test]
    fn the_menu_changes_in_place_never_replaced() {
        let quiet = tray_model(&calm(), &names);
        // Figures only: every line keeps its place; only words change.
        let mut busier = calm();
        busier.machine.cpu_pct = Some(71.0);
        let next = tray_model(&busier, &names);
        let steps = plan(&quiet.items, &next.items);
        assert!(steps.remove.is_empty() && steps.insert.is_empty());
        assert_eq!(
            steps.retext,
            [(1, "CPU 71% · Memory 21 GB of 32 GB".to_string())]
        );
        assert_eq!(apply(&quiet.items, &next.items, &steps), next.items);
        // A heavy session appears: the start and the end stay; the run in
        // between is replaced (the first line and the new section).
        let mut heavy_mac = calm();
        heavy_mac.heavy = vec![heavy("s-1", "CPU 96%"), heavy("s-2", "3.2 GB")];
        let heavy_model = tray_model(&heavy_mac, &names);
        let steps = plan(&quiet.items, &heavy_model.items);
        assert_eq!(
            apply(&quiet.items, &heavy_model.items, &steps),
            heavy_model.items
        );
        assert!(
            steps.remove.end <= quiet.items.len() - 3,
            "Open This Mac, Quit and their divider stay put"
        );
        // And back.
        let steps = plan(&heavy_model.items, &quiet.items);
        assert_eq!(apply(&heavy_model.items, &quiet.items, &steps), quiet.items);
        // To and from "not measured".
        let steps = plan(&quiet.items, &unmeasured_model().items);
        assert_eq!(
            apply(&quiet.items, &unmeasured_model().items, &steps),
            unmeasured_model().items
        );
    }

    #[test]
    fn voiceover_hears_what_the_number_counts_and_words_for_unknowns() {
        let mut info = calm();
        info.heavy = vec![heavy("s-1", "CPU 96%"), heavy("s-2", "3.2 GB")];
        let model = tray_model(&info, &names);
        assert!(model.tooltip.starts_with("AI Juggler — 2 need a glance: "));
        info.heavy.truncate(1);
        assert!(
            tray_model(&info, &names)
                .tooltip
                .starts_with("AI Juggler — 1 needs a glance: ")
        );
        let mut unknown = calm();
        unknown.machine.cpu_pct = None;
        assert_eq!(
            tray_model(&unknown, &names).items[1].text(),
            "CPU unknown · Memory 21 GB of 32 GB"
        );
        let stale = unmeasured_model();
        assert_eq!(stale.items[0].text(), "Not measured right now");
        assert_eq!(stale.title, None);
    }

    fn alert(
        key: &str,
        kind: &str,
        title: &str,
        session: Option<&str>,
        place: Option<&str>,
    ) -> OverloadAlertInfo {
        OverloadAlertInfo {
            key: key.into(),
            kind: kind.into(),
            title: title.into(),
            detail: "detail".into(),
            session_id: session.map(str::to_string),
            place: place.map(str::to_string),
            subject_bytes: None,
            subject_estimate: false,
            subject_cpu: Some(96.0),
            since_ms: 0,
        }
    }
}
