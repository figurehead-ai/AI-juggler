//! 0.5.2 G56: a test's stand-in app (a fake `Claude.app` whose program a
//! test runs) that macOS accepts. A program put in `<name>.app/Contents/
//! MacOS/` by hand and run from there makes macOS check the whole bundle;
//! one with no `Info.plist` and no seal over its resources fails ("code has
//! no resources but signature indicates they must be present", -67056) and
//! macOS shows "“<name>” is damaged and can't be opened" on the screen of
//! whoever is at the Mac. A presence test's copy of `bash` as
//! `Claude.app/Contents/MacOS/Claude` raised it twice during the 0.5.2 RW1
//! walks. `scripts/stand_in_app_lint.py` keeps every test on this fixture.
//! Shared by every crate's tests:
//! `#[path = "…/juggler-supervisor/tests/fixtures/stand_in_app.rs"] mod stand_in_app;`

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Makes `bundle` (a folder named `<stem>.app`) a whole stand-in app with
/// `program` as its `Contents/MacOS/<exe>`, and returns that program's path:
///
/// - an `Info.plist` naming it "AIJ Test <stem>", id `test.aij-stand-in.…`
///   (never a real app's name or id, so the system never takes it for one);
/// - the program copied by `/bin/cp -X` (no extended attributes; written
///   by a child, so no writer of this process keeps it busy: AJ-E806);
/// - an ad-hoc signature over the whole bundle (`identifier` names the
///   code when a test needs it to, never the bundle's id), checked with
///   `codesign --verify --strict` before anything runs from it.
///
/// # Panics
///
/// When any step fails: no stand-in that macOS would call damaged is ever
/// left to run.
pub fn make(bundle: &Path, exe: &str, program: &Path, identifier: Option<&str>) -> PathBuf {
    let stem = bundle
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|_| bundle.extension().is_some_and(|ext| ext == "app"))
        .expect("a stand-in app is a folder named <name>.app");
    let macos = bundle.join("Contents/MacOS");
    std::fs::create_dir_all(&macos).expect("the stand-in's Contents/MacOS");
    let target = macos.join(exe);
    run(
        Command::new("/bin/cp").arg("-X").arg(program).arg(&target),
        "copy the stand-in's program",
    );
    let ident: String = stem
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    std::fs::write(
        bundle.join("Contents/Info.plist"),
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
             <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \
             \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
             <plist version=\"1.0\"><dict>\n\
             <key>CFBundleExecutable</key><string>{exe}</string>\n\
             <key>CFBundleIdentifier</key><string>test.aij-stand-in.{ident}</string>\n\
             <key>CFBundleName</key><string>AIJ Test {stem}</string>\n\
             <key>CFBundleDisplayName</key><string>AIJ Test {stem}</string>\n\
             <key>CFBundlePackageType</key><string>APPL</string>\n\
             <key>LSUIElement</key><true/>\n\
             </dict></plist>\n"
        ),
    )
    .expect("the stand-in's Info.plist");
    let mut sign = Command::new("/usr/bin/codesign");
    sign.args(["--force", "--sign", "-"]);
    if let Some(identifier) = identifier {
        sign.args(["--identifier", identifier]);
    }
    run(sign.arg(bundle), "sign the stand-in app as a whole");
    assert!(
        verified(bundle),
        "{} is a whole signed bundle (else macOS calls it damaged)",
        bundle.display()
    );
    target
}

/// Whether `bundle` passes `codesign --verify --strict`: the check whose
/// failure makes macOS call an app damaged when its program runs.
pub fn verified(bundle: &Path) -> bool {
    Command::new("/usr/bin/codesign")
        .args(["--verify", "--strict"])
        .arg(bundle)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

fn run(command: &mut Command, what: &str) {
    let out = command.stdin(Stdio::null()).output();
    assert!(out.is_ok(), "{what}: {:?}", out.err());
    let out = out.expect("checked just above");
    assert!(
        out.status.success(),
        "{what}: {}",
        String::from_utf8_lossy(&out.stderr).trim()
    );
}
