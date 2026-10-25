//! 0.1.5 live check: Go to it's activation on the real Mac.
//!
//! Opens a throwaway TextEdit of its own (`open -n -g`, a new instance in
//! the background, found by a marker argument, so its pid is exactly the
//! one this test started), puts this test process in front as far as
//! macOS allows, brings the TextEdit forward by its pid through the
//! production path ([`juggler_app::activation`]: hand over, activate from
//! here, then read the frontmost app), and checks the answer against what
//! is really in front. Then it quits that TextEdit (`kill` on its pid
//! only) and no other app is touched: never the user's own TextEdit,
//! Claude or ChatGPT windows.
//!
//! AppKit wants the main thread, which the test harness keeps for itself,
//! so this test has its own `main` (`harness = false`) and runs only when
//! asked:
//!
//! `cargo test -p juggler-app --test activation_real -- --ignored`
//!
//! It needs a logged-in desktop session (not CI) and no permission.

// Test crates are pragmatic by policy (QUALITY.md §2).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|arg| arg == "--list") {
        return;
    }
    if !args
        .iter()
        .any(|arg| arg == "--ignored" || arg == "--include-ignored")
    {
        println!("activation_real: ignored (drives a real TextEdit; run with `-- --ignored`)");
        return;
    }
    #[cfg(target_os = "macos")]
    mac::run();
    #[cfg(not(target_os = "macos"))]
    println!("activation_real: macOS only");
}

#[cfg(target_os = "macos")]
mod mac {
    use std::process::Command;
    use std::time::{Duration, Instant};

    use juggler_app::activation::{self, Activation, Front, Request};
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy, NSRunningApplication};
    use objc2_foundation::{NSDate, NSRunLoop};

    /// Runs the main thread's loop for `span`: AppKit learns which app is
    /// in front only while it runs.
    fn spin(span: Duration) {
        let until = NSDate::dateWithTimeIntervalSinceNow(span.as_secs_f64());
        NSRunLoop::currentRunLoop().runUntilDate(&until);
    }

    /// The production Mac, waiting with the loop running (the app's own
    /// main thread runs it; here the test is that thread).
    struct Spinning;

    impl Front for Spinning {
        fn request(&self, pid: u32) -> Request {
            activation::Mac.request(pid)
        }

        fn frontmost(&self) -> Option<u32> {
            activation::Mac.frontmost()
        }

        fn pause(&self, span: Duration) {
            spin(span);
        }
    }

    /// The throwaway TextEdit: quit on drop, whatever happened.
    struct Throwaway(u32);

    impl Drop for Throwaway {
        fn drop(&mut self) {
            let pid = self.0.to_string();
            let _ = Command::new("/bin/kill").args(["-TERM", &pid]).status();
            let deadline = Instant::now() + Duration::from_secs(5);
            while alive(self.0) && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(100));
            }
            if alive(self.0) {
                let _ = Command::new("/bin/kill").args(["-KILL", &pid]).status();
                std::thread::sleep(Duration::from_millis(300));
            }
            println!("throwaway TextEdit {pid} quit: {}", !alive(self.0));
        }
    }

    fn alive(pid: u32) -> bool {
        Command::new("/bin/kill")
            .args(["-0", &pid.to_string()])
            .status()
            .is_ok_and(|status| status.success())
    }

    /// Opens a new TextEdit in the background, marked, and finds its pid.
    fn open_throwaway() -> Throwaway {
        let marker = format!("aij-activation-probe-{}", std::process::id());
        let opened = Command::new("/usr/bin/open")
            .args([
                "-n",
                "-g",
                "-a",
                "TextEdit",
                "--args",
                "-ApplePersistenceIgnoreState",
                "YES",
                "-NSShowAppCentricOpenPanelInsteadOfUntitledFile",
                "NO",
                "-AIJugglerProbe",
                &marker,
            ])
            .status()
            .expect("open runs");
        assert!(opened.success(), "open -n -g -a TextEdit failed");
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            let out = Command::new("/usr/bin/pgrep")
                .args(["-f", &marker])
                .output()
                .expect("pgrep runs");
            let pids: Vec<u32> = String::from_utf8_lossy(&out.stdout)
                .lines()
                .filter_map(|line| line.trim().parse().ok())
                .collect();
            if let [pid] = pids[..] {
                return Throwaway(pid);
            }
            assert!(
                pids.len() < 2,
                "more than one process carries the marker: {pids:?}"
            );
            assert!(Instant::now() < deadline, "TextEdit didn't start");
            spin(Duration::from_millis(100));
        }
    }

    /// Asks for this test process to be the active app, as far as macOS
    /// lets a process started from a shell; says whether it is.
    fn take_front(app: &NSApplication) -> bool {
        if objc2::available!(macos = 14.0) {
            app.activate();
        } else {
            #[allow(deprecated)]
            app.activateIgnoringOtherApps(true);
        }
        let deadline = Instant::now() + Duration::from_secs(2);
        while !app.isActive() && Instant::now() < deadline {
            spin(Duration::from_millis(50));
        }
        app.isActive()
    }

    pub fn run() {
        let mtm = MainThreadMarker::new().expect("the test's own main thread");
        let app = NSApplication::sharedApplication(mtm);
        // A regular app (a Dock icon while it runs), so it can be active.
        app.setActivationPolicy(NSApplicationActivationPolicy::Regular);
        app.finishLaunching();
        let me = std::process::id();
        let target = open_throwaway();
        let pid = target.0;
        println!("throwaway TextEdit: pid {pid}; this test: pid {me}");
        // Launched: AppKit knows it.
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let ready = NSRunningApplication::runningApplicationWithProcessIdentifier(
                i32::try_from(pid).unwrap(),
            )
            .is_some_and(|running| running.isFinishedLaunching());
            if ready {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "TextEdit didn't finish launching"
            );
            spin(Duration::from_millis(100));
        }
        let in_front = take_front(&app);
        let before = Spinning.frontmost();
        println!("before: this test in front: {in_front}; frontmost pid {before:?}");
        assert_ne!(before, Some(pid), "the throwaway must start behind");

        let started = Instant::now();
        let result = activation::bring(&Spinning, pid);
        let took = started.elapsed();
        // What is really in front, after another moment.
        spin(Duration::from_millis(200));
        let after = Spinning.frontmost();
        println!("bring({pid}) = {result:?} in {took:?}; frontmost now {after:?}");

        drop(target);
        // The answer agrees with the Mac: forward only when it was in front.
        match result {
            Activation::Forward => assert_eq!(after, Some(pid), "said forward, wasn't"),
            Activation::Behind(why) => {
                assert_ne!(after, Some(pid), "said behind, was forward");
                println!(
                    "honest outcome: {}",
                    activation::behind_text("TextEdit", why)
                );
            }
            Activation::Gone => panic!("the throwaway was running"),
        }
        // With this process in front, the hand-over must work.
        if in_front {
            assert_eq!(result, Activation::Forward, "handed over, didn't come");
        }
        println!("activation_real: ok");
    }
}
