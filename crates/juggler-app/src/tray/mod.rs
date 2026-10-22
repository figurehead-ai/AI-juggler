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
use crate::machine_health::{ThisMacInfo, ToolTreeInfo};
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

/// A tool's CPU in Home's words and measure: "CPU 14%" where 100% is one
/// core busy (Activity Monitor's count), "CPU —" when unmeasured.
fn cpu_words(cpu: Option<f64>) -> String {
    cpu.map_or_else(|| "CPU —".to_string(), |cpu| format!("CPU {cpu:.0}%"))
}

/// Memory as the menu says it: "about" when the sum is an estimate.
fn memory_words(bytes: u64, estimate: bool) -> String {
    if estimate {
        format!("about {}", bytes_words(bytes))
    } else {
        bytes_words(bytes)
    }
}

/// Who an alert is about, and what of theirs is high — as Home says it.
fn alert_subject(
    alert: &OverloadAlertInfo,
    name_of: &dyn Fn(&str) -> Option<String>,
) -> Option<String> {
    // A session as the rail names it; a tool AI Juggler didn't start in
    // the backend's words for it ("The Claude app").
    let who = match &alert.session_id {
        Some(id) => short(&name_of(id).unwrap_or_else(|| "a session".into())),
        None => alert.subject_name.clone()?,
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
    // Calm states in Home's words ("Getting tight", "Plenty of room"):
    // what macOS reports belongs to an alert, when the number shows.
    let summary = &info.machine.summary;
    let text = if info.tools.count == 0 && summary.level == "plenty" {
        "Nothing running — your Mac has room".to_string()
    } else {
        summary.words.clone()
    };
    (text, TrayAction::ThisMac)
}

/// The Mac's own figures in one line, in Home's words and order: memory
/// in use, then the processor as a share of all its cores.
fn machine_line(info: &ThisMacInfo) -> String {
    let machine = &info.machine;
    if !machine.sampled {
        return "Measuring this Mac…".to_string();
    }
    let memory = match machine.memory_used {
        Some(used) => format!(
            "Memory: {} of {} in use",
            bytes_words(used),
            bytes_words(machine.memory_total)
        ),
        None => format!("Memory: {}", bytes_words(machine.memory_total)),
    };
    let cpu = machine.cpu_pct.map_or_else(
        || "CPU —".to_string(),
        |cpu| format!("{cpu:.0}% of all CPU cores"),
    );
    format!("{memory} · {cpu}")
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
    // Each tool as Home's rows read: memory, then CPU of one core.
    for group in &info.by_tool {
        items.push(TrayItem::Label(format!(
            "{} · {} · {} · {}",
            group.label,
            group.count_words,
            memory_words(group.totals.rss_bytes, group.totals.memory_estimate),
            cpu_words(group.cpu_one_core_pct),
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
/// attention shape adds a small dot in the top-right corner, outside the
/// arc (a mark, never read as a fourth ball), so the state is a shape,
/// never colour alone.
#[must_use]
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "pixel maths on a 36 px icon: coverage in 0..=1 times 255 fits a u8"
)]
pub fn icon_rgba(attention: bool) -> Vec<u8> {
    // (centre x, centre y, radius) in pixels.
    let mut discs: Vec<(f32, f32, f32)> =
        vec![(8.0, 25.0, 4.5), (18.0, 13.0, 4.5), (28.0, 25.0, 4.5)];
    if attention {
        discs.push((32.0, 4.0, 3.0));
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
/// Storage failures; a lost blocking task.
#[tauri::command]
pub async fn tray_get_shown(
    state: tauri::State<'_, std::sync::Arc<crate::state::AppState>>,
) -> crate::error::AppResult<Option<bool>> {
    if !cfg!(target_os = "macos") {
        return Ok(None);
    }
    let state = std::sync::Arc::clone(&state);
    tauri::async_runtime::spawn_blocking(move || shown(&state).map(Some))
        .await
        .map_err(|_| crate::error::AppError::StatePoisoned)?
}

/// Shows or hides the menu-bar item, now and at the next launch. Async,
/// so it never runs on the main thread the status item's calls wait for.
///
/// # Errors
///
/// Storage failures; a lost blocking task.
#[tauri::command]
pub async fn tray_set_shown(
    app: tauri::AppHandle,
    state: tauri::State<'_, std::sync::Arc<crate::state::AppState>>,
    shown: bool,
) -> crate::error::AppResult<()> {
    let state = std::sync::Arc::clone(&state);
    tauri::async_runtime::spawn_blocking(move || store_shown(&state, shown))
        .await
        .map_err(|_| crate::error::AppError::StatePoisoned)??;
    #[cfg(target_os = "macos")]
    set_shown(&app, shown);
    #[cfg(not(target_os = "macos"))]
    let _ = app;
    Ok(())
}

/// Whether the user left the menu-bar item on (the default).
fn shown(state: &crate::state::AppState) -> crate::error::AppResult<bool> {
    let storage = state
        .storage
        .lock()
        .map_err(|_| crate::error::AppError::StatePoisoned)?;
    Ok(storage.app_settings().get(SETTING_KEY)?.as_deref() != Some("off"))
}

fn store_shown(state: &crate::state::AppState, shown: bool) -> crate::error::AppResult<()> {
    let storage = state
        .storage
        .lock()
        .map_err(|_| crate::error::AppError::StatePoisoned)?;
    Ok(storage
        .app_settings()
        .set(SETTING_KEY, if shown { "on" } else { "off" })?)
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

    use crate::EVENT_TRAY_OPEN;

    /// A status-item call that failed is told, never dropped. (Until the
    /// app has its tracing subscriber, standard error is its log.)
    fn note<T>(result: tauri::Result<T>, what: &str) {
        if let Err(error) = result {
            eprintln!("ai-juggler: menu bar: {what} failed: {error}");
        }
    }

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
            note(icon.set_visible(shown), "show or hide");
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
        if last.title != model.title {
            note(icon.set_title(model.title.as_deref()), "title");
        }
        if last.attention != model.attention {
            note(icon.set_icon(Some(image(model.attention))), "icon");
            note(icon.set_icon_as_template(true), "template icon");
        }
        if last.tooltip != model.tooltip {
            note(icon.set_tooltip(Some(&model.tooltip)), "tooltip");
        }
        let steps = plan(&last.items, &model.items);
        for (position, words) in &steps.retext {
            if let Some(Entry::Line(line)) = entries.get(*position) {
                note(line.set_text(words), "menu line");
            }
        }
        for _ in steps.remove.clone() {
            note(menu.remove_at(steps.remove.start), "remove a line");
        }
        let mut inserted = Vec::new();
        for (offset, item) in model.items[steps.insert.clone()].iter().enumerate() {
            let entry = match make(app, serial, item) {
                Ok(entry) => entry,
                Err(error) => {
                    note::<()>(Err(error), "make a line");
                    continue;
                }
            };
            serial += 1;
            let at = steps.insert.start + offset;
            note(
                match &entry {
                    Entry::Line(line) => menu.insert(line, at),
                    Entry::Divider(divider) => menu.insert(divider, at),
                },
                "insert a line",
            );
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
mod tests;
