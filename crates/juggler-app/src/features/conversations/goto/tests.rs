//! Go to it over a scripted Mac: each outcome, no prompt without the
//! setting, and "forward" only once the app is frontmost.

use std::sync::Mutex;

use juggler_adapter::live::HostApp;

use super::*;

#[derive(Default)]
struct FakeMac {
    gone: bool,
    refused: bool,
    restarted: bool,
    /// macOS takes the request, and another app stays in front.
    behind: bool,
    /// AI Juggler wasn't the active app when it asked.
    not_in_front: bool,
    front: Mutex<Option<u32>>,
    windows: Option<(usize, usize)>,
    log: Mutex<Vec<String>>,
}

impl FakeMac {
    fn log(&self) -> Vec<String> {
        self.log.lock().unwrap().clone()
    }
}

impl Front for FakeMac {
    fn request(&self, pid: u32) -> Request {
        self.log.lock().unwrap().push(format!("activate {pid}"));
        if self.refused {
            return Request::Refused;
        }
        if !self.behind {
            *self.front.lock().unwrap() = Some(pid);
        }
        Request::Asked {
            in_front: !self.not_in_front,
        }
    }

    fn frontmost(&self) -> Option<u32> {
        // AI Juggler (pid 1) until the target comes forward.
        Some(self.front.lock().unwrap().unwrap_or(1))
    }

    fn pause(&self, _span: Duration) {}
}

impl Desktop for FakeMac {
    fn start_of(&self, pid: u32) -> Option<i64> {
        (!self.gone).then_some(i64::from(pid) + if self.restarted { 999 } else { 0 })
    }

    fn open_link(&self, link: &str) -> bool {
        self.log.lock().unwrap().push(format!("open {link}"));
        true
    }

    fn open_folder(&self, command: &str, folder: &Path) -> bool {
        self.log
            .lock()
            .unwrap()
            .push(format!("run {command} {}", folder.display()));
        true
    }

    fn osascript(&self, source: &str) -> Result<String, String> {
        self.log.lock().unwrap().push("osascript".to_owned());
        if source.contains("System Events") {
            let (all, minimised) = self.windows.ok_or("no")?;
            return Ok(format!("{all},{minimised}"));
        }
        Ok("ok".to_owned())
    }

    fn tty(&self, _pid: u32) -> Option<String> {
        Some("/dev/ttys003".to_owned())
    }
}

fn hosts() -> HostRegistry {
    let mut apps = juggler_adapters::claude_live::host_apps();
    apps.extend(juggler_adapters::codex_live::host_apps());
    HostRegistry::new(apps)
}

fn target(
    app_id: Option<&str>,
    name: &str,
    kind: Option<HostKind>,
    instance: Option<&str>,
) -> Target {
    Target {
        session_id: "019a1b2c-3d4e-7f80-9a1b-2c3d4e5f6a7b".to_owned(),
        cwd: Some(PathBuf::from("/Users/you/Dev/my app")),
        place: Place {
            pid: 12,
            start: Some(1),
            host: Some(Host {
                app_id: app_id.map(str::to_owned),
                name: name.to_owned(),
                kind,
                pid: 10,
                start: Some(10),
                instance_dir: None,
                instance: instance.map(str::to_owned),
            }),
            profile: None,
        },
    }
}

fn claude() -> Target {
    target(
        Some("claude-app"),
        "Claude",
        Some(HostKind::Desktop),
        Some("Work"),
    )
}

#[test]
fn a_claude_instance_comes_forward_by_its_process_and_says_where_to_look() {
    let mac = FakeMac::default();
    let done = go(&claude(), &hosts(), &mac, Gates::default());
    assert_eq!(
        done.outcome,
        "Brought Claude · Work forward; pick the session in its sidebar"
    );
    assert_eq!(done.reached, "app");
    assert_eq!(
        mac.log(),
        ["activate 10"],
        "by pid, and nothing else: no prompt"
    );
}

#[test]
fn window_counts_and_minimised_need_window_focus_turned_on() {
    let gates = Gates {
        windows: true,
        tabs: false,
    };
    let three = FakeMac {
        windows: Some((3, 1)),
        ..FakeMac::default()
    };
    assert_eq!(
        go(&claude(), &hosts(), &three, gates).outcome,
        "Brought Claude · Work forward (3 windows); pick the session in its sidebar"
    );
    let minimised = FakeMac {
        windows: Some((1, 1)),
        ..FakeMac::default()
    };
    assert_eq!(
        go(&claude(), &hosts(), &minimised, gates).outcome,
        "Claude · Work is minimised — click it in the Dock"
    );
    // Off: System Events is never asked.
    let off = FakeMac {
        windows: Some((3, 1)),
        ..FakeMac::default()
    };
    let _ = go(&claude(), &hosts(), &off, Gates::default());
    assert!(!off.log().iter().any(|line| line == "osascript"));
}

#[test]
fn a_codex_thread_opens_by_its_link() {
    let mac = FakeMac::default();
    let done = go(
        &target(Some("codex-app"), "ChatGPT", Some(HostKind::Desktop), None),
        &hosts(),
        &mac,
        Gates::default(),
    );
    assert_eq!(done.outcome, "Opened the thread in ChatGPT");
    assert_eq!(done.reached, "thread");
    assert_eq!(
        mac.log(),
        [
            "activate 10",
            "open codex://threads/019a1b2c-3d4e-7f80-9a1b-2c3d4e5f6a7b"
        ]
    );
}

#[test]
fn an_editor_opens_the_folder_and_never_an_extension_link() {
    let mac = FakeMac::default();
    let done = go(
        &target(Some("cursor"), "Cursor", Some(HostKind::Editor), None),
        &hosts(),
        &mac,
        Gates::default(),
    );
    assert_eq!(done.outcome, "Opened my app in Cursor");
    assert_eq!(
        mac.log(),
        ["activate 10", "open cursor://file//Users/you/Dev/my%20app"]
    );
    let code = FakeMac::default();
    let _ = go(
        &target(
            Some("vscode"),
            "Visual Studio Code",
            Some(HostKind::Editor),
            None,
        ),
        &hosts(),
        &code,
        Gates::default(),
    );
    assert_eq!(
        code.log(),
        ["activate 10", "run code /Users/you/Dev/my app"]
    );
    assert!(!code.log().iter().any(|line| line.contains("vscode://")));
}

#[test]
fn a_terminal_tab_is_selected_only_when_tabs_are_on() {
    let off = FakeMac::default();
    let done = go(
        &target(Some("terminal"), "Terminal", Some(HostKind::Terminal), None),
        &hosts(),
        &off,
        Gates::default(),
    );
    assert_eq!(
        done.outcome,
        "Brought Terminal forward; the session is in one of its tabs"
    );
    assert_eq!(
        off.log(),
        ["activate 10"],
        "no AppleScript without the setting"
    );
    let on = FakeMac::default();
    let done = go(
        &target(Some("terminal"), "Terminal", Some(HostKind::Terminal), None),
        &hosts(),
        &on,
        Gates {
            windows: false,
            tabs: true,
        },
    );
    assert_eq!(
        done.outcome,
        "Brought Terminal forward and selected its tab"
    );
    assert_eq!(done.reached, "tab");
}

#[test]
fn a_gone_reused_or_refused_app_says_so() {
    let gone = FakeMac {
        gone: true,
        ..FakeMac::default()
    };
    assert_eq!(
        go(&claude(), &hosts(), &gone, Gates::default()).outcome,
        "Claude · Work isn't open now"
    );
    assert!(gone.log().is_empty());
    let reused = FakeMac {
        restarted: true,
        ..FakeMac::default()
    };
    assert_eq!(
        go(&claude(), &hosts(), &reused, Gates::default()).outcome,
        "Claude · Work isn't open now",
        "a reused pid is never brought forward"
    );
    assert!(reused.log().is_empty());
    let refused = FakeMac {
        refused: true,
        ..FakeMac::default()
    };
    assert_eq!(
        go(&claude(), &hosts(), &refused, Gates::default()).reached,
        "none"
    );
    let mut nowhere = claude();
    nowhere.place.host = None;
    assert_eq!(
        go(&nowhere, &hosts(), &FakeMac::default(), Gates::default()).reached,
        "none"
    );
}

#[test]
fn a_slow_step_answers_within_the_limit() {
    let answer = within(Duration::from_millis(50), || {
        // poll: a slow step stood in for, bounded at 400 ms.
        std::thread::sleep(Duration::from_millis(400));
        1
    });
    assert_eq!(answer, None);
    assert_eq!(within(LIMIT, || 2), Some(2));
}

#[test]
fn a_places_open_brings_its_running_app_forward_and_never_a_reused_pid() {
    let place = target(
        Some("claude-app"),
        "Claude",
        Some(HostKind::Desktop),
        Some("b-work"),
    )
    .place;
    let host = place.host.unwrap();
    let mac = FakeMac::default();
    let said = bring_forward(&host, &mac);
    assert_eq!(said.reached, "app");
    assert_eq!(said.outcome, "Brought Claude · b-work forward");
    assert_eq!(mac.log(), vec!["activate 10".to_owned()]);
    // The app quit and its pid went to another process: nothing is touched.
    let reused = FakeMac {
        restarted: true,
        ..FakeMac::default()
    };
    let said = bring_forward(&host, &reused);
    assert_eq!(said.reached, "none");
    assert!(reused.log().is_empty(), "never activates another process");
    let refused = FakeMac {
        refused: true,
        ..FakeMac::default()
    };
    assert!(
        bring_forward(&host, &refused)
            .outcome
            .contains("Click it in the Dock")
    );
    // Asked, and another app stayed in front: never "Brought … forward".
    let behind = FakeMac {
        behind: true,
        ..FakeMac::default()
    };
    let said = bring_forward(&host, &behind);
    assert_eq!(
        said.outcome,
        "macOS didn't bring Claude · b-work forward. Click it in the Dock"
    );
    assert_eq!(said.reached, "none");
}

#[test]
fn an_app_that_stays_behind_is_never_said_to_be_forward() {
    let behind = FakeMac {
        behind: true,
        ..FakeMac::default()
    };
    let said = go(&claude(), &hosts(), &behind, Gates::default());
    assert_eq!(
        said.outcome,
        "macOS didn't bring Claude · Work forward. Click it in the Dock"
    );
    assert_eq!(said.reached, "none");
    // The reason, when it's known: AI Juggler had nothing to hand over.
    let not_in_front = FakeMac {
        behind: true,
        not_in_front: true,
        ..FakeMac::default()
    };
    assert_eq!(
        go(&claude(), &hosts(), &not_in_front, Gates::default()).outcome,
        "macOS didn't bring Claude · Work forward: AI Juggler wasn't in front. Click it in the Dock"
    );
    // A request that came forward anyway is forward, with or without it.
    let lucky = FakeMac {
        not_in_front: true,
        ..FakeMac::default()
    };
    assert_eq!(
        go(&claude(), &hosts(), &lucky, Gates::default()).outcome,
        "Brought Claude · Work forward; pick the session in its sidebar"
    );
}

#[test]
fn a_codex_thread_that_opened_behind_says_both() {
    let behind = FakeMac {
        behind: true,
        ..FakeMac::default()
    };
    let said = go(
        &target(Some("codex-app"), "ChatGPT", Some(HostKind::Desktop), None),
        &hosts(),
        &behind,
        Gates::default(),
    );
    assert_eq!(
        said.outcome,
        "Opened the thread in ChatGPT, but macOS didn't bring it forward. Click it in the Dock"
    );
    assert_eq!(said.reached, "thread");
    // A terminal's tab: selected, and the terminal still behind.
    let tab = go(
        &target(Some("terminal"), "Terminal", Some(HostKind::Terminal), None),
        &hosts(),
        &FakeMac {
            behind: true,
            ..FakeMac::default()
        },
        Gates {
            windows: false,
            tabs: true,
        },
    );
    assert_eq!(
        tab.outcome,
        "Selected its tab in Terminal, but macOS didn't bring it forward. Click it in the Dock"
    );
}

#[test]
fn the_registry_has_no_extension_session_link() {
    let registry = hosts();
    let vscode: &HostApp = registry.by_id("vscode").unwrap();
    assert!(vscode.thread_link.is_none());
    assert!(vscode.folder_link.is_none());
}
