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

/// `builder`, kept on the app's own pages and with popups refused. Every
/// window the app makes goes through this.
#[must_use]
pub fn guarded<R: Runtime, M: Manager<R>>(
    builder: WebviewWindowBuilder<'_, R, M>,
) -> WebviewWindowBuilder<'_, R, M> {
    builder
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
    let made = WebviewWindowBuilder::from_config(app, &config).and_then(|b| guarded(b).build());
    made.map(|_| ()).map_err(|error| AppError::Io {
        context: "make the window",
        message: error.to_string(),
    })
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
        (Opened::ByApp, "http") => cfg!(debug_assertions) && matches!(host, "localhost" | "127.0.0.1"),
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
pub fn open_web<R: Runtime>(app: &tauri::AppHandle<R>, url: impl AsRef<str>, opened: Opened) -> AppResult<()> {
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
        assert_eq!(page_allowed(&url("http://localhost:5173/")), cfg!(debug_assertions));
    }

    #[test]
    fn a_clicked_link_opens_any_web_address_and_nothing_else() {
        assert_eq!(refusal("https://docs.rs/serde", Opened::ByUser), None);
        assert_eq!(refusal("http://example.com/a?b", Opened::ByUser), None);
        for bad in ["file:///Users/x/.ssh/id_rsa", "javascript:alert(1)", "ssh://host", "not a url", "x-apple.systempreferences:"] {
            assert!(refusal(bad, Opened::ByUser).is_some(), "{bad} must be refused");
        }
    }

    #[test]
    fn the_app_opens_only_https() {
        assert_eq!(refusal("https://github.com/figurehead-ai/AI-juggler", Opened::ByApp), None);
        assert!(refusal("http://github.com/", Opened::ByApp).is_some());
        assert!(refusal("file:///tmp/x.html", Opened::ByApp).is_some());
        assert_eq!(refusal("http://localhost:8787/verify", Opened::ByApp).is_none(), cfg!(debug_assertions));
    }
}
