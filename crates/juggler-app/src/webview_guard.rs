//! What AI Juggler's web views may load and open (AJ-E2003).
//!
//! Text from a transcript or another agent ends up on screen, so a page
//! that some day runs a script it shouldn't must still get nowhere: every
//! window of the app is built through [`guarded`], which keeps it on the
//! app's own pages (no navigation away, no `window.open` popups — a
//! `WebView2` window would otherwise open one). Links meant for the browser
//! go through [`open_web`], from Rust, never by the page navigating.

use tauri::webview::NewWindowResponse;
use tauri::{Manager, Runtime, Url, WebviewWindowBuilder};

use crate::error::{AppError, AppResult};
use crate::window_frame::Choice;

/// Whether a web view may show `url`: only the app's own pages (the
/// bundled assets; in a debug build, Vite's dev server too).
#[must_use]
pub fn page_allowed(url: &Url) -> bool {
    match (url.scheme(), url.host_str()) {
        // macOS and Linux serve the bundle as tauri://localhost, Windows
        // as http(s)://tauri.localhost.
        ("tauri", Some("localhost")) | ("http" | "https", Some("tauri.localhost")) => true,
        ("http", Some("localhost")) => cfg!(debug_assertions) && url.port() == Some(5173),
        _ => false,
    }
}

/// The colour a window paints before its page does, and wherever the page
/// has nothing on screen (AJ-E1549): the dark theme's canvas,
/// `--color-canvas` in `ui/src/styles.css` (dark is the default theme).
/// Without one, the web view keeps its own opaque white under the
/// page, and a window shown again, or seen again after the page was
/// suspended behind another window, flashed white for a frame. The page
/// sets its theme's own colour at run time (`ui/src/theme.ts`).
pub const BASE_COLOUR: tauri::window::Color = tauri::window::Color(0x0b, 0x0e, 0x14, 0xff);

/// `builder`, kept on the app's own pages and with popups refused, on the
/// base colour. Every window the app makes goes through this.
#[must_use]
pub fn guarded<R: Runtime, M: Manager<R>>(
    builder: WebviewWindowBuilder<'_, R, M>,
) -> WebviewWindowBuilder<'_, R, M> {
    builder
        .background_color(BASE_COLOUR)
        .on_navigation(page_allowed)
        .on_new_window(|_, _| NewWindowResponse::Deny)
}

/// Makes the main window from `tauri.conf.json`'s one window entry (which
/// has `create: false`, so Tauri doesn't make an unguarded one first).
///
/// # Errors
///
/// [`AppError::Io`] when the window can't be made.
pub fn create_main<R: Runtime>(app: &tauri::AppHandle<R>) -> AppResult<()> {
    let config = app
        .config()
        .app
        .windows
        .first()
        .cloned()
        .ok_or_else(|| AppError::Io {
            context: "make the window",
            message: "tauri.conf.json has no window".to_owned(),
        })?;
    let mut config = main_config(config, crate::test_launch::headless());
    let state = app.try_state::<std::sync::Arc<crate::state::AppState>>();
    // 0.5.1 F21: the theme the user chose and the frame it was left at.
    let (choice, frame) = state.as_ref().map_or((Choice::Dark, None), |state| {
        crate::window_frame::kept(state)
    });
    if let Some(saved) = frame {
        let least = (
            config.min_width.unwrap_or(0.0),
            config.min_height.unwrap_or(0.0),
        );
        let screens = crate::window_frame::screens(app);
        let (width, height, at) = crate::window_frame::placed(saved, least, &screens);
        (config.width, config.height) = (width, height);
        if let Some((x, y)) = at {
            (config.x, config.y, config.center) = (Some(x), Some(y), false);
        }
    }
    // "As macOS shows": the window is made hidden, given the system's
    // colour, then shown, so its first frame is that colour too.
    let show_after = choice == Choice::System && config.visible;
    if show_after {
        config.visible = false;
    }
    // rc32: what the window keeps in its storage is this data folder's
    let scope = state.map_or_else(
        || "window.__AIJ_STORE_SCOPE__ = \"\";".to_owned(),
        |state| crate::lab_scope::store_scope_script(&state.data_dir),
    );
    let made = WebviewWindowBuilder::from_config(app, &config).and_then(|b| {
        guarded(b)
            .background_color(choice.colour(tauri::Theme::Dark))
            .initialization_script(&scope)
            .build()
    });
    let window = made.map_err(|error| AppError::Io {
        context: "make the window",
        message: error.to_string(),
    })?;
    if choice == Choice::System {
        let system = window.theme().unwrap_or(tauri::Theme::Dark);
        let _ = window.set_background_color(Some(choice.colour(system)));
        if show_after {
            let _ = window.show();
            let _ = window.set_focus();
        }
    }
    Ok(())
}

/// The main window's settings from `tauri.conf.json`'s entry, as it's
/// made: a click on it while another window (the Floor) is key acts at
/// once, as a desktop app's does (0.4.1 rc33: Tauri's default left
/// `acceptsFirstMouse` off, so the first click back from the Floor only
/// made the window key and the second did the action). A test's launch
/// never shows it (`src/test_launch.rs`).
fn main_config(
    mut config: tauri::utils::config::WindowConfig,
    headless: bool,
) -> tauri::utils::config::WindowConfig {
    config.accept_first_mouse = true;
    if headless {
        config.visible = false;
        config.focus = false;
    }
    config
}

/// Who asked for a link to open in the browser.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Opened {
    /// The user clicked it (a terminal's link): any http or https address.
    ByUser,
    /// The app opens an address of its own (a constant, an adapter's
    /// sign-in page, the account service's): https only (a debug build's
    /// local account service excepted). The page never supplies these, so
    /// no host list is kept here: the hosts belong to their adapters.
    ByApp,
}

/// Why `url` may not be opened, or `None` when it may.
#[must_use]
pub fn refusal(url: &str, opened: Opened) -> Option<&'static str> {
    let Ok(parsed) = Url::parse(url) else {
        return Some("That isn't a web address.");
    };
    let host = parsed.host_str().unwrap_or_default();
    let web = match (opened, parsed.scheme()) {
        (Opened::ByUser, "http" | "https") | (Opened::ByApp, "https") => !host.is_empty(),
        (Opened::ByApp, "http") => {
            cfg!(debug_assertions) && matches!(host, "localhost" | "127.0.0.1")
        }
        _ => false,
    };
    (!web).then_some("Only web addresses open in the browser.")
}

/// Opens `url` in the browser, after [`refusal`] lets it through.
///
/// # Errors
///
/// [`AppError::Validation`] for an address that may not be opened;
/// [`AppError::Io`] when the system can't open the browser.
pub fn open_web<R: Runtime>(
    app: &tauri::AppHandle<R>,
    url: impl AsRef<str>,
    opened: Opened,
) -> AppResult<()> {
    use tauri_plugin_opener::OpenerExt as _;
    let url = url.as_ref();
    if let Some(message) = refusal(url, opened) {
        return Err(AppError::Validation {
            subject: "link",
            message: message.to_owned(),
        });
    }
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|error| AppError::Io {
            context: "open the browser",
            message: error.to_string(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(text: &str) -> Url {
        Url::parse(text).expect("a url")
    }

    #[test]
    fn a_window_stays_on_the_apps_own_pages() {
        assert!(page_allowed(&url("tauri://localhost/index.html")));
        assert!(page_allowed(&url("http://tauri.localhost/index.html")));
        assert!(page_allowed(&url("https://tauri.localhost/")));
        for away in [
            "https://example.com/",
            "http://localhost:8080/",
            "file:///etc/passwd",
            "javascript:alert(1)",
            "data:text/html,<script>1</script>",
            "tauri://evil/",
            "https://tauri.localhost.example.com/",
        ] {
            assert!(!page_allowed(&url(away)), "{away} must be refused");
        }
        // Vite's dev server, in a debug build only.
        assert_eq!(
            page_allowed(&url("http://localhost:5173/")),
            cfg!(debug_assertions)
        );
    }

    /// 0.4.1 rc33: the first click back in the main window from the
    /// Floor only activated it. The main window acts on that first click,
    /// as made from the app's own config entry.
    #[test]
    fn the_main_window_acts_on_the_first_click() {
        let config: tauri::utils::config::Config =
            serde_json::from_str(include_str!("../tauri.conf.json")).expect("the app's config");
        let entry = config.app.windows.first().cloned().expect("a window entry");
        let made = main_config(entry, false);
        assert!(made.accept_first_mouse);
        assert!(made.visible && made.focus, "shown, outside a test launch");
        let hidden = main_config(made, true);
        assert!(hidden.accept_first_mouse && !hidden.visible && !hidden.focus);
    }

    /// AJ-E1549: the main window is made on the base colour (its config
    /// entry, which the page's tokens are checked against in
    /// `ui/tests/bundle/theme-base.test.ts`), so nothing under the page is white.
    #[test]
    fn the_main_window_is_made_on_the_base_colour() {
        let config: tauri::utils::config::Config =
            serde_json::from_str(include_str!("../tauri.conf.json")).expect("the app's config");
        let entry = config.app.windows.first().expect("a window entry");
        assert_eq!(entry.background_color, Some(BASE_COLOUR));
        assert_eq!(BASE_COLOUR.3, 0xff, "opaque: nothing behind it shows");
    }

    #[test]
    fn a_clicked_link_opens_any_web_address_and_nothing_else() {
        assert_eq!(refusal("https://docs.rs/serde", Opened::ByUser), None);
        assert_eq!(refusal("http://example.com/a?b", Opened::ByUser), None);
        for bad in [
            "file:///Users/x/.ssh/id_rsa",
            "javascript:alert(1)",
            "ssh://host",
            "not a url",
            "x-apple.systempreferences:",
        ] {
            assert!(
                refusal(bad, Opened::ByUser).is_some(),
                "{bad} must be refused"
            );
        }
    }

    #[test]
    fn the_app_opens_only_https() {
        assert_eq!(
            refusal("https://github.com/figurehead-ai/AI-juggler", Opened::ByApp),
            None
        );
        assert!(refusal("http://github.com/", Opened::ByApp).is_some());
        assert!(refusal("file:///tmp/x.html", Opened::ByApp).is_some());
        assert_eq!(
            refusal("http://localhost:8787/verify", Opened::ByApp).is_none(),
            cfg!(debug_assertions)
        );
    }
}
