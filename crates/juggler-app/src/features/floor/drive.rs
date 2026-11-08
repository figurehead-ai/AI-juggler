//! The Floor's test input (AJ-E3514's persona walk): only in a build with
//! the `floor-harness` feature, which no shipped build has
//! (`scripts/floor_harness_check.sh`, and `release_seams_check.sh` on the
//! shipped binary), and only when [`VAR`] names a folder that doesn't
//! exist yet. The app makes it owner-only and listens there on a socket;
//! `scripts/qa/floor_drive` sends one line per action and reads back the
//! time to the first frame drawn after the Floor took it in.
//!
//! Posted system events don't reach the Floor's window, so the walk's
//! testers act through this instead: the same `Input`s the window's
//! event monitor makes, into the same render thread.
//!
//! Lines (window points from the top left):
//! `open`, `click X Y`, `move X Y`, `drag X1 Y1 X2 Y2`, `rdrag X1 Y1 X2 Y2`,
//! `scroll X Y DY`, `zoom X Y DY`, `key NAME`. The answer: `ok N ms` or
//! `error: …`.

use std::io::{BufRead as _, BufReader, Write as _};
use std::os::unix::fs::{DirBuilderExt as _, PermissionsExt as _};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use juggler_floor::driver::{Input, Key};
use tauri::Manager as _;

use super::render::{COUNTERS, Msg};
use super::window;

/// The variable naming the folder for the socket.
pub const VAR: &str = "AIJ_FLOOR_INPUT";
/// The socket's name in that folder.
pub const SOCKET: &str = "floor.sock";
/// The longest wait for the Floor to take an input in or draw after it.
const WAIT: Duration = Duration::from_secs(3);
/// Steps in a drag (the window's monitor sends one per pointer move).
const DRAG_STEPS: u16 = 12;

/// Starts listening when [`VAR`] is set (and the keychain is off: a lab
/// or a replica, never the user's own data).
pub fn start(app: &tauri::AppHandle) {
    let Some(dir) = std::env::var_os(VAR).map(PathBuf::from) else {
        return;
    };
    if std::env::var("AIJ_NO_KEYCHAIN").as_deref() != Ok("1") {
        tracing::warn!("floor input: refused without AIJ_NO_KEYCHAIN=1 (labs and replicas only)");
        return;
    }
    let listener = match bind(&dir) {
        Ok(listener) => listener,
        Err(e) => {
            tracing::warn!("floor input: {e}");
            return;
        }
    };
    let app = app.clone();
    let _ = std::thread::Builder::new()
        .name("floor-input".into())
        .spawn(move || {
            for stream in listener.incoming().flatten() {
                serve(&app, stream);
            }
        });
}

/// Makes `dir` (it must not exist: nobody else made it or can write in
/// it), owner-only, and binds the socket there, owner-only too.
fn bind(dir: &Path) -> Result<UnixListener, String> {
    // macOS keeps a socket's path in 104 bytes, its end included.
    if dir.join(SOCKET).as_os_str().len() >= 104 {
        return Err(format!(
            "{} is too long a path for a socket (use a short one, like /tmp/aij-floor-in)",
            dir.display()
        ));
    }
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(dir)
        .map_err(|e| format!("{} must be a new folder: {e}", dir.display()))?;
    let path = dir.join(SOCKET);
    let listener = UnixListener::bind(&path).map_err(|e| format!("can't listen: {e}"))?;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
        .map_err(|e| format!("can't make the socket owner-only: {e}"))?;
    Ok(listener)
}

fn serve(app: &tauri::AppHandle, stream: UnixStream) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let mut line = String::new();
    let Ok(mut reader) = stream.try_clone().map(BufReader::new) else {
        return;
    };
    if reader.read_line(&mut line).is_err() {
        return;
    }
    let answer = match parse(line.trim()) {
        Ok(Action::Open) => match window::open(app) {
            Ok(()) => wait_frame(COUNTERS.frames.load(Ordering::Relaxed), Instant::now()),
            Err(e) => format!("error: {e}"),
        },
        Ok(Action::Inputs(inputs)) => act(app, &inputs),
        Ok(Action::FrontMain) => {
            let handle = app.clone();
            let _ = app.run_on_main_thread(move || crate::main_window::bring_forward(&handle));
            "ok 0 ms".to_owned()
        }
        Ok(Action::View(None)) => view_items(app),
        Ok(Action::View(Some(choice))) => view_choose(app, &choice),
        Err(e) => format!("error: {e}"),
    };
    let mut stream = stream;
    let _ = writeln!(stream, "{answer}");
}

/// Sends the inputs; answers with the time to the first frame drawn after
/// the Floor took the last of them in.
fn act(app: &tauri::AppHandle, inputs: &[Input]) -> String {
    let start = Instant::now();
    let taken = COUNTERS.inputs.load(Ordering::Relaxed);
    for input in inputs {
        window::send(app, Msg::Input(*input));
    }
    let want = taken + inputs.len() as u64;
    while COUNTERS.inputs.load(Ordering::Relaxed) < want {
        if start.elapsed() > WAIT {
            return "error: the Floor isn't open (open it first)".into();
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    wait_frame(COUNTERS.frames.load(Ordering::Relaxed), start)
}

fn wait_frame(frames: u64, start: Instant) -> String {
    while COUNTERS.frames.load(Ordering::Relaxed) <= frames {
        if start.elapsed() > WAIT {
            return "error: no frame drawn in 3 s (is the Floor covered?)".into();
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    format!("ok {} ms", start.elapsed().as_millis())
}

/// The View menu's items as it would show them, ✓ on the ones chosen.
fn view_items(app: &tauri::AppHandle) -> String {
    let state = std::sync::Arc::clone(
        app.state::<std::sync::Arc<crate::state::AppState>>()
            .inner(),
    );
    let settings = state
        .storage
        .lock()
        .ok()
        .and_then(|s| super::store::Settings::read(&s).ok())
        .unwrap_or_default();
    let on = window::view_words(&settings);
    let mut lines = Vec::new();
    for ((title, _, choices), on) in window::VIEW.iter().zip(on) {
        lines.push(format!("{title}:"));
        for (word, words) in *choices {
            lines.push(format!("  {} {words}", if *word == on { "✓" } else { " " }));
        }
    }
    lines.push("Settings › The Floor…".to_owned());
    format!("ok 0 ms\n{}", lines.join("\n"))
}

/// Chooses a View menu item by its words ("Labels > Attention only", or
/// "Settings › The Floor…"), as a click on it would.
fn view_choose(app: &tauri::AppHandle, choice: &str) -> String {
    let wanted: Vec<String> = choice
        .split('>')
        .map(|part| part.trim().to_lowercase())
        .collect();
    let id = if wanted.first().is_some_and(|w| w.starts_with("settings")) {
        Some(format!("{}settings", window::VIEW_PREFIX))
    } else {
        window::VIEW.iter().find_map(|(title, _, choices)| {
            let item = match wanted.as_slice() {
                [section, item] if section == &title.to_lowercase() => item,
                [item] => item,
                _ => return None,
            };
            choices
                .iter()
                .find(|(_, words)| words.to_lowercase() == *item)
                .map(|(word, _)| format!("{}{}:{word}", window::VIEW_PREFIX, title.to_lowercase()))
        })
    };
    let Some(id) = id else {
        return format!("error: no View item \"{choice}\" (say `view` to list them)");
    };
    let start = Instant::now();
    let frames = COUNTERS.frames.load(Ordering::Relaxed);
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || window::view_chosen(&handle, &id));
    wait_frame(frames, start)
}

#[derive(Debug, PartialEq)]
enum Action {
    Open,
    Inputs(Vec<Input>),
    /// The View menu's items, or one chosen ("Labels > Attention only").
    View(Option<String>),
    /// The main window forward, as the app brings it (a window manager
    /// swaps it in for the app, not for an outside click).
    FrontMain,
}

fn parse(line: &str) -> Result<Action, String> {
    let mut words = line.split_whitespace();
    let verb = words.next().ok_or("say an action")?;
    let rest: Vec<&str> = words.collect();
    let n = |i: usize| -> Result<f32, String> {
        rest.get(i)
            .and_then(|w| w.parse::<f32>().ok())
            .filter(|v| v.is_finite())
            .ok_or_else(|| format!("{verb} needs numbers"))
    };
    let inputs = match verb {
        "open" => return Ok(Action::Open),
        "front-main" => return Ok(Action::FrontMain),
        "view" => {
            let choice = rest.join(" ");
            return Ok(Action::View((!choice.is_empty()).then_some(choice)));
        }
        "click" => vec![
            Input::Move { x: n(0)?, y: n(1)? },
            Input::Click { x: n(0)?, y: n(1)? },
        ],
        "move" => vec![Input::Move { x: n(0)?, y: n(1)? }],
        "drag" | "rdrag" => {
            let (x1, y1, x2, y2) = (n(0)?, n(1)?, n(2)?, n(3)?);
            let steps = f32::from(DRAG_STEPS);
            let mut inputs = vec![Input::Move { x: x1, y: y1 }];
            inputs.extend((0..DRAG_STEPS).map(|_| Input::Drag {
                dx: (x2 - x1) / steps,
                dy: (y2 - y1) / steps,
                rotate: verb == "rdrag",
            }));
            inputs
        }
        "scroll" | "zoom" => vec![Input::Scroll {
            dx: 0.0,
            dy: n(2)?,
            precise: verb == "scroll",
            x: n(0)?,
            y: n(1)?,
            zoom: verb == "zoom",
        }],
        "key" => vec![Input::Key(key(rest.first().copied().unwrap_or(""))?)],
        other => return Err(format!("unknown action {other}")),
    };
    Ok(Action::Inputs(inputs))
}

fn key(name: &str) -> Result<Key, String> {
    Ok(match name {
        "left" => Key::Left,
        "right" => Key::Right,
        "up" => Key::Up,
        "down" => Key::Down,
        "tab" => Key::Tab,
        "backtab" => Key::BackTab,
        "return" | "enter" => Key::Enter,
        "pageup" => Key::PageUp,
        "pagedown" => Key::PageDown,
        "escape" => Key::Escape,
        one if one.chars().count() == 1 => {
            Key::Char(one.chars().next().map_or(' ', |c| c.to_ascii_lowercase()))
        }
        other => return Err(format!("unknown key {other}")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_become_the_windows_own_inputs() {
        assert_eq!(parse("open"), Ok(Action::Open));
        assert_eq!(
            parse("click 10 20"),
            Ok(Action::Inputs(vec![
                Input::Move { x: 10.0, y: 20.0 },
                Input::Click { x: 10.0, y: 20.0 }
            ]))
        );
        let turn = match parse("rdrag 0 0 120 -24") {
            Ok(Action::Inputs(turn)) => turn,
            _ => Vec::new(),
        };
        assert_eq!(turn.len(), 1 + usize::from(DRAG_STEPS));
        assert_eq!(
            turn[1],
            Input::Drag {
                dx: 10.0,
                dy: -2.0,
                rotate: true
            }
        );
        assert_eq!(
            parse("key H"),
            Ok(Action::Inputs(vec![Input::Key(Key::Char('h'))]))
        );
        assert_eq!(
            parse("key pagedown"),
            Ok(Action::Inputs(vec![Input::Key(Key::PageDown)]))
        );
    }

    #[test]
    fn bad_lines_are_refused_in_words() {
        assert!(parse("").is_err());
        assert!(parse("click 1").is_err());
        assert!(parse("click 1 NaN").is_err());
        assert!(parse("key f13").is_err());
        assert!(parse("rm -rf /").is_err());
        assert_eq!(parse("view"), Ok(Action::View(None)));
        assert_eq!(
            parse("view Labels > Attention only"),
            Ok(Action::View(Some("Labels > Attention only".to_owned())))
        );
    }

    #[test]
    fn the_folder_must_be_new_and_ends_owner_only() {
        let base = tempfile::tempdir().expect("a temp folder");
        let dir = base.path().join("in");
        let listener = bind(&dir).expect("binds in a new folder");
        let mode = |p: &Path| std::fs::metadata(p).map(|m| m.permissions().mode() & 0o777);
        assert_eq!(mode(&dir).ok(), Some(0o700));
        assert_eq!(mode(&dir.join(SOCKET)).ok(), Some(0o600));
        drop(listener);
        assert!(bind(&dir).is_err(), "an existing folder is refused");
        let long = base.path().join("x".repeat(110));
        assert!(bind(&long).is_err_and(|e| e.contains("too long")));
        assert!(!long.exists(), "nothing is made for a path that can't work");
    }
}
