//! Test doubles the places tests share (D-94): a starter that records
//! what it was asked and a desktop that does nothing unless told.

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use juggler_platform::apps::Start;

use super::start;
use crate::activation::{Front, Request};
use crate::features::conversations::goto::Desktop;

/// A starter that starts nothing for real: it records each start and
/// quit it was asked for.
#[derive(Default)]
pub(crate) struct Recorder {
    pub starts: Mutex<Vec<(PathBuf, Start, Vec<String>)>>,
    /// The env each start passed.
    pub envs: Mutex<Vec<Vec<(String, String)>>>,
    /// 0.3.8: a running process lost its window.
    pub lost: bool,
    /// It won't quit when asked.
    pub stays: bool,
    /// The processes asked to quit, in order (never really signalled).
    pub quits: Mutex<Vec<u32>>,
}

impl start::Starter for Recorder {
    fn trusted(&self, _: &Path) -> Result<(), String> {
        Ok(())
    }

    fn start(&self, launch: &start::Launch) -> Result<(), String> {
        self.starts
            .lock()
            .unwrap()
            .push((launch.bundle.clone(), launch.how, launch.args.clone()));
        self.envs.lock().unwrap().push(launch.env.clone());
        Ok(())
    }

    fn detached(&self, _: u32, _: Option<i64>) -> bool {
        self.lost
    }

    fn quit(&self, pid: u32) -> Result<(), String> {
        self.quits.lock().unwrap().push(pid);
        Ok(())
    }

    fn alive(&self, pid: u32, _: Option<i64>) -> bool {
        self.stays || !self.quits.lock().unwrap().contains(&pid)
    }
}

/// A desktop with no windows: nothing comes to the front, no folder
/// opens and no script runs. `opens_links` records and opens links;
/// `lost` reads real process starts and calls every process detached.
#[derive(Default)]
pub(crate) struct FakeDesktop {
    pub links: Mutex<Vec<String>>,
    /// AJ-E1610: the links sent to one process (`pid`, link), when
    /// `sends_links` (else none goes, as before).
    pub sent: Mutex<Vec<(u32, String)>>,
    pub sends_links: bool,
    pub opens_links: bool,
    pub lost: bool,
}

impl FakeDesktop {
    /// One that opens links and records them.
    pub(crate) fn opening_links() -> Self {
        Self {
            opens_links: true,
            ..Self::default()
        }
    }

    /// 0.3.8: a desktop that finds the app by its process but macOS has
    /// no app for it: it runs on with no window.
    pub(crate) fn lost() -> Self {
        Self {
            lost: true,
            ..Self::default()
        }
    }
}

impl Front for FakeDesktop {
    fn request(&self, _: u32) -> Request {
        Request::Gone
    }
    fn frontmost(&self) -> Option<u32> {
        None
    }
    fn pause(&self, _: Duration) {}
}

impl Desktop for FakeDesktop {
    fn start_of(&self, pid: u32) -> Option<i64> {
        if !self.lost {
            return None;
        }
        crate::features::conversations::procs::some(&[pid])
            .get(&pid)
            .and_then(|process| process.start)
    }
    fn detached(&self, _: u32, _: Option<i64>) -> bool {
        self.lost
    }
    fn open_link(&self, link: &str) -> bool {
        if self.opens_links {
            self.links.lock().unwrap().push(link.to_owned());
        }
        self.opens_links
    }
    fn open_folder(&self, _: &str, _: &Path) -> bool {
        false
    }
    fn osascript(&self, _: &str) -> Result<String, String> {
        Err("never".into())
    }
    fn tty(&self, _: u32) -> Option<String> {
        None
    }
    fn send_link_to(
        &self,
        pid: u32,
        link: &str,
        _: bool,
    ) -> crate::features::conversations::goto::LinkSent {
        use crate::features::conversations::goto::LinkSent;
        if !self.sends_links {
            return LinkSent::Failed;
        }
        self.sent.lock().unwrap().push((pid, link.to_owned()));
        LinkSent::Sent
    }
}

/// A state over `root` whose places see a made-up Mac: only this test's
/// own process runs on it (its pid and start are real, so a place pinned
/// to it is found alive), and its home is `root/home`. Nothing the test
/// asserts then depends on what runs on the machine (a real ChatGPT or
/// Codex app was found and brought forward instead of started).
pub(crate) fn on_made_up_mac(
    state: crate::state::AppState,
    root: &Path,
) -> std::sync::Arc<crate::state::AppState> {
    let me = std::process::id();
    let _ = state.places.made_up.set(super::MadeUpMac {
        table: crate::features::conversations::procs::some(&[me]),
        home: root.join("home"),
    });
    std::sync::Arc::new(state)
}
