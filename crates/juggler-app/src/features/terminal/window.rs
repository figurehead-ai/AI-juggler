//! A terminal in its own window (AJ-E1506, wireframe 05): the same PTY,
//! the same scrollback, in a real window the user can put on another
//! display. The session stays supervised in the main window, which says
//! where its terminal went and can bring it back. One view at a time
//! drives the PTY's size, so the main window shows no second copy.

use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent};

use crate::error::{AppError, AppResult};

/// Event: the terminals now in their own windows (payload: their ids).
pub const EVENT_TERMINAL_WINDOWS: &str = "terminal://windows";

/// A terminal window's label: this, then the terminal's id. The webview
/// reads its own label to know it is one (capabilities/terminal-window.json).
pub const LABEL_PREFIX: &str = "terminal-";

/// The longest window title (the session's name, then the app's).
const MAX_TITLE: usize = 120;

/// A terminal's window label, if its id can be one (letters, digits, `-`
/// and `_`: every session and login id AI Juggler makes).
#[must_use]
pub fn label_of(id: &str) -> Option<String> {
    let fits = !id.is_empty()
        && id.len() <= 200
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_');
    fits.then(|| format!("{LABEL_PREFIX}{id}"))
}

/// The ids of the terminals in their own windows, but `closing`'s.
fn popped_out(app: &AppHandle, closing: Option<&str>) -> Vec<String> {
    let mut ids: Vec<String> = app
        .webview_windows()
        .into_keys()
        .filter(|label| Some(label.as_str()) != closing)
        .filter_map(|label| label.strip_prefix(LABEL_PREFIX).map(str::to_owned))
        .collect();
    ids.sort();
    ids
}

/// The terminals in their own windows now.
#[must_use]
pub fn windows(app: &AppHandle) -> Vec<String> {
    popped_out(app, None)
}

fn announce(app: &AppHandle, closing: Option<&str>) {
    let _ = app.emit(EVENT_TERMINAL_WINDOWS, popped_out(app, closing));
}

/// Opens terminal `id` in its own window, or brings its window forward.
///
/// # Errors
///
/// [`AppError::Refused`] for an id that can't name a window;
/// [`AppError::Io`] when the window can't be made.
pub fn pop_out(app: &AppHandle, id: &str, title: &str) -> AppResult<()> {
    let Some(label) = label_of(id) else {
        return Err(AppError::Refused(
            "That terminal can't have its own window".into(),
        ));
    };
    if let Some(window) = app.get_webview_window(&label) {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        return Ok(());
    }
    let name: String = title
        .chars()
        .filter(|c| !c.is_control())
        .take(MAX_TITLE)
        .collect();
    let window = WebviewWindowBuilder::new(app, &label, WebviewUrl::App("index.html".into()))
        .title(if name.trim().is_empty() {
            "AI Juggler".to_owned()
        } else {
            format!("{name} — AI Juggler")
        })
        .inner_size(960.0, 620.0)
        .min_inner_size(480.0, 320.0)
        .build()
        .map_err(|error| AppError::Io {
            context: "open the terminal's window",
            message: error.to_string(),
        })?;
    let handle = app.clone();
    let closing = label.clone();
    window.on_window_event(move |event| {
        if matches!(event, WindowEvent::Destroyed) {
            announce(&handle, Some(&closing));
        }
    });
    announce(app, None);
    Ok(())
}

/// Closes terminal `id`'s own window: its terminal shows in the main
/// window again. The session keeps running either way.
pub fn pop_in(app: &AppHandle, id: &str) {
    if let Some(window) = label_of(id).and_then(|label| app.get_webview_window(&label)) {
        let _ = window.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_plain_ids_name_a_window() {
        assert_eq!(
            label_of("sess-0b7f_1").as_deref(),
            Some("terminal-sess-0b7f_1")
        );
        assert_eq!(
            label_of("login-123456").as_deref(),
            Some("terminal-login-123456")
        );
        for refused in ["", "a/b", "a:b", "a b", "ü", &"x".repeat(201)] {
            assert_eq!(label_of(refused), None, "{refused}");
        }
    }
}
