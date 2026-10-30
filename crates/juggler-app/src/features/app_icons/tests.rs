//! D-38: the checks before an app's bundle is read, the cache, and how an
//! app is found. The live test (ignored) reads three apps every Mac has.

use std::cell::Cell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use juggler_adapter::live::{HostApp, HostKind};

use super::bundle::{self, Ask, Bundle, Lookup, OWN_APP_ID};
use super::cache::IconCache;
use super::{SIZES, icon_data_url, render};
use crate::features::cache::Gate;
use crate::features::conversations::places::HostRegistry;

/// A Mac made of fixtures.
#[derive(Default)]
struct Fake {
    exe: HashMap<u32, PathBuf>,
    own: Option<PathBuf>,
    ids: HashMap<String, PathBuf>,
    home: Option<PathBuf>,
}

impl Lookup for Fake {
    fn exe_of(&self, pid: u32) -> Option<PathBuf> {
        self.exe.get(&pid).cloned()
    }
    fn own_exe(&self) -> Option<PathBuf> {
        self.own.clone()
    }
    fn app_with_id(&self, id: &str) -> Option<PathBuf> {
        self.ids.get(id).cloned()
    }
    fn home(&self) -> Option<PathBuf> {
        self.home.clone()
    }
    fn bundle_id(&self, _path: &Path) -> Option<String> {
        None
    }
}

/// A bundle at `root/name` with an Info.plist and an icon file.
/// The cache's gate, on.
fn gate() -> std::sync::Arc<Gate> {
    std::sync::Arc::new(Gate::default())
}

fn app(root: &Path, name: &str) -> PathBuf {
    let path = root.join(name);
    std::fs::create_dir_all(path.join("Contents/Resources")).unwrap();
    std::fs::create_dir_all(path.join("Contents/MacOS")).unwrap();
    std::fs::write(path.join("Contents/Info.plist"), b"<plist/>").unwrap();
    std::fs::write(path.join("Contents/Resources/AppIcon.icns"), b"icns").unwrap();
    std::fs::write(path.join("Contents/MacOS/app"), b"").unwrap();
    path
}

fn hosts() -> HostRegistry {
    HostRegistry::new(vec![HostApp {
        id: "an-app".into(),
        name: "An App".into(),
        bundle: "An App.app".into(),
        bundle_id: Some("dev.example.an-app".into()),
        kind: HostKind::Desktop,
        instance_arg: None,
        default_instance: None,
        thread_link: None,
        folder_link: None,
        folder_command: None,
        tab_script: None,
    }])
}

const PNG: &[u8] = b"\x89PNG\r\n\x1a\nrest";

// Links: Unix only (a Windows link needs a privilege).
#[cfg(unix)]
#[test]
fn only_an_app_folder_that_is_not_a_link_is_read() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().canonicalize().unwrap();
    let good = app(&root, "Good.app");
    assert_eq!(bundle::checked(&good), Some(good.clone()));
    // Not a bundle: a plain folder, a file named like one, a bare `.app`,
    // a relative path, a path that isn't there.
    std::fs::create_dir_all(root.join("Folder")).unwrap();
    assert_eq!(bundle::checked(&root.join("Folder")), None);
    std::fs::write(root.join("File.app"), b"").unwrap();
    assert_eq!(bundle::checked(&root.join("File.app")), None);
    std::fs::create_dir_all(root.join(".app")).unwrap();
    assert_eq!(bundle::checked(&root.join(".app")), None);
    assert_eq!(bundle::checked(Path::new("Good.app")), None);
    assert_eq!(bundle::checked(&root.join("Gone.app")), None);
    // A link named like a bundle, even to a real one, is refused.
    std::os::unix::fs::symlink(&good, root.join("Link.app")).unwrap();
    assert_eq!(bundle::checked(&root.join("Link.app")), None);
    std::os::unix::fs::symlink(&root, root.join("Up.app")).unwrap();
    assert_eq!(bundle::checked(&root.join("Up.app")), None);
}

#[cfg(unix)]
#[test]
fn a_bundle_whose_icon_or_plist_links_out_of_it_is_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().canonicalize().unwrap();
    let secret = root.join("secret.txt");
    std::fs::write(&secret, b"not an icon").unwrap();

    let icns = app(&root, "Icns.app");
    let at = icns.join("Contents/Resources/AppIcon.icns");
    std::fs::remove_file(&at).unwrap();
    std::os::unix::fs::symlink(&secret, &at).unwrap();
    assert_eq!(bundle::checked(&icns), None);

    let plist = app(&root, "Plist.app");
    std::fs::remove_file(plist.join("Contents/Info.plist")).unwrap();
    std::os::unix::fs::symlink(&secret, plist.join("Contents/Info.plist")).unwrap();
    assert_eq!(bundle::checked(&plist), None);

    let resources = app(&root, "Resources.app");
    std::fs::remove_dir_all(resources.join("Contents/Resources")).unwrap();
    std::os::unix::fs::symlink(&root, resources.join("Contents/Resources")).unwrap();
    assert_eq!(bundle::checked(&resources), None);

    // A link that stays inside the bundle is fine.
    let inside = app(&root, "Inside.app");
    std::fs::write(inside.join("Contents/Resources/Real.icns"), b"icns").unwrap();
    std::os::unix::fs::symlink(
        inside.join("Contents/Resources/Real.icns"),
        inside.join("Contents/Resources/Alias.icns"),
    )
    .unwrap();
    assert_eq!(bundle::checked(&inside), Some(inside.clone()));
}

#[test]
fn a_cache_key_is_a_plain_file_name() {
    assert_eq!(bundle::safe_key("com.apple.Terminal"), "com.apple.Terminal");
    assert_eq!(bundle::safe_key("../../etc/passwd"), "etcpasswd");
    assert_eq!(bundle::safe_key("Visual Studio Code"), "VisualStudioCode");
    assert_eq!(bundle::safe_key(".."), "");
}

#[test]
fn an_app_is_found_by_its_place_its_id_or_its_folder_and_never_otherwise() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().canonicalize().unwrap();
    let registered = app(&root.join("Applications"), "An App.app");
    let elsewhere = app(&root.join("Elsewhere"), "An App.app");
    let other = app(&root, "Other.app");
    let own = app(&root, "AI Juggler.app");
    let mut mac = Fake {
        own: Some(own.join("Contents/MacOS/app")),
        home: Some(root.clone()),
        ..Fake::default()
    };
    mac.exe.insert(7, elsewhere.join("Contents/MacOS/app"));
    mac.exe.insert(8, other.join("Contents/MacOS/app"));
    let hosts = hosts();
    let ask = |id: Option<&str>, pid: Option<u32>| Ask {
        app_id: id.map(str::to_owned),
        app_pid: pid,
    };
    let path = |found: Option<Bundle>| found.map(|bundle| bundle.path);

    // AI Juggler's own bundle.
    assert_eq!(
        path(bundle::locate(&ask(Some(OWN_APP_ID), None), &hosts, &mac)),
        Some(own.clone())
    );
    // A running place's bundle, as its process runs it.
    assert_eq!(
        path(bundle::locate(&ask(Some("an-app"), Some(7)), &hosts, &mac)),
        Some(elsewhere.clone())
    );
    // An app the registry doesn't list, by its process alone.
    assert_eq!(
        path(bundle::locate(&ask(None, Some(8)), &hosts, &mac)),
        Some(other.clone())
    );
    // A process in another bundle than the id names is passed over: the
    // registry's app is found in the user's Applications instead.
    assert_eq!(
        path(bundle::locate(&ask(Some("an-app"), Some(8)), &hosts, &mac)),
        Some(registered.clone())
    );
    // By its bundle id first, when macOS knows it.
    mac.ids
        .insert("dev.example.an-app".into(), elsewhere.clone());
    assert_eq!(
        path(bundle::locate(&ask(Some("an-app"), None), &hosts, &mac)),
        Some(elsewhere.clone())
    );
    // An id nobody registered, or nothing at all, finds nothing.
    assert_eq!(
        bundle::locate(&ask(Some("nobody"), None), &hosts, &mac),
        None
    );
    assert_eq!(bundle::locate(&ask(None, None), &hosts, &mac), None);
    // The key is the bundle's name when macOS gives no bundle id.
    assert_eq!(
        bundle::locate(&ask(None, Some(8)), &hosts, &mac)
            .unwrap()
            .key,
        "Other"
    );
    // A process outside any bundle has no icon.
    mac.exe.insert(9, root.join("bin/tool"));
    assert_eq!(bundle::locate(&ask(None, Some(9)), &hosts, &mac), None);
}

#[test]
fn an_icon_is_kept_until_the_app_changes_then_read_again() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().canonicalize().unwrap();
    let path = app(&root, "Kept.app");
    let dir = root.join("data/icons");
    let renders = Cell::new(0);
    let render = || {
        renders.set(renders.get() + 1);
        Some(PNG.to_vec())
    };
    let first = Bundle {
        version: bundle::version_of(&path),
        path: path.clone(),
        key: "dev.example.kept".into(),
    };
    let cache = IconCache::new(dir.clone(), gate());
    assert_eq!(cache.get_or_render(&first, 64, render), Some(PNG.to_vec()));
    assert_eq!(cache.get_or_render(&first, 64, render), Some(PNG.to_vec()));
    assert_eq!(renders.get(), 1, "the second read is from memory");
    let file = cache.file_of(&first, 64);
    assert!(file.ends_with(format!("dev.example.kept-{}-64.icon", first.version)));
    let kept = std::fs::read(&file).unwrap();
    assert!(kept.ends_with(PNG), "the icon, after the cache's head line");
    // A new run reads the file, not the app.
    let again = IconCache::new(dir.clone(), gate());
    assert_eq!(again.get_or_render(&first, 64, render), Some(PNG.to_vec()));
    assert_eq!(renders.get(), 1);

    // The app is updated: its bundle changed, so its icon is read again
    // and the old version's file goes.
    let later = SystemTime::now() + Duration::from_secs(3600);
    std::fs::File::options()
        .write(true)
        .open(path.join("Contents/Info.plist"))
        .unwrap()
        .set_modified(later)
        .unwrap();
    let updated = Bundle {
        version: bundle::version_of(&path),
        ..first.clone()
    };
    assert!(updated.version > first.version);
    assert_eq!(
        again.get_or_render(&updated, 64, render),
        Some(PNG.to_vec())
    );
    assert_eq!(renders.get(), 2);
    assert!(!file.exists(), "the old version's icon is removed");
    assert!(again.file_of(&updated, 64).exists());

    // What can't be read is no icon, and isn't written.
    let none = IconCache::new(root.join("none"), gate());
    assert_eq!(none.get_or_render(&first, 32, || None), None);
    assert_eq!(
        none.get_or_render(&first, 128, || Some(b"not a png".to_vec())),
        None
    );
    assert!(!root.join("none").exists());
    // A kept file that isn't a PNG is read again from the app.
    std::fs::write(cache.file_of(&first, 32), b"junk").unwrap();
    let fresh = IconCache::new(dir, gate());
    assert_eq!(fresh.get_or_render(&first, 32, render), Some(PNG.to_vec()));
    assert_eq!(renders.get(), 3);
}

#[test]
fn only_the_three_sizes_are_read() {
    let mac = Fake::default();
    let tmp = tempfile::tempdir().unwrap();
    let cache = IconCache::new(tmp.path().join("icons"), gate());
    let ask = Ask {
        app_id: Some(OWN_APP_ID.into()),
        app_pid: None,
    };
    assert_eq!(SIZES, [32, 64, 128]);
    for px in [0, 16, 48, 256, 4096] {
        assert_eq!(icon_data_url(&cache, &ask, px, &hosts(), &mac), None);
    }
}

#[test]
fn the_command_is_async_and_runs_off_the_main_thread() {
    let source = include_str!("mod.rs");
    let command = source
        .split("#[tauri::command]")
        .nth(1)
        .expect("the command");
    assert!(
        command
            .trim_start()
            .starts_with("pub async fn app_icons_read(")
    );
    assert!(command.contains("crate::ipc::blocking("));
}

#[test]
fn pixels_are_written_as_a_png_and_resampled_cleanly() {
    let red = [255u8, 0, 0, 255];
    let clear = [0u8, 0, 0, 0];
    // 4 × 4: the left half red, the right half clear.
    let pixels: Vec<u8> = (0..16)
        .flat_map(|at| if at % 4 < 2 { red } else { clear })
        .collect();
    let half = render::resample(&pixels, 4, 4, 2);
    assert_eq!(half, [red, clear, red, clear].concat());
    // A clear neighbour never darkens an edge.
    let one = render::resample(&pixels, 4, 4, 1);
    assert_eq!(&one[..3], &[255, 0, 0]);
    assert_eq!(one[3], 128);
    let png = render::encode_png(&pixels, 4, 4).unwrap();
    assert!(super::cache::is_png(&png));
    assert_eq!(render::encode_png(&pixels, 3, 4), None);
    assert_eq!(render::byte(2.0), 255);
    assert_eq!(render::byte(-1.0), 0);
}

/// Reads Finder's, TextEdit's and Terminal's icons on this Mac, only
/// reading their bundles: `cargo test -p juggler-app --lib
/// app_icons::tests::live -- --ignored`. With `AIJ_ICON_OUT` set, the
/// PNGs are also written there to be looked at.
#[test]
#[ignore = "reads apps installed on this Mac"]
fn live_icons_of_installed_apps() {
    let tmp = tempfile::tempdir().unwrap();
    let cache = IconCache::new(tmp.path().join("icons"), gate());
    let hosts = HostRegistry::new(Vec::new());
    let out = std::env::var_os("AIJ_ICON_OUT").map(PathBuf::from);
    for (name, path) in [
        ("Finder", "/System/Library/CoreServices/Finder.app"),
        ("TextEdit", "/System/Applications/TextEdit.app"),
        ("Terminal", "/System/Applications/Utilities/Terminal.app"),
    ] {
        let found = bundle::checked(Path::new(path));
        assert!(found.is_some(), "{name} isn't a bundle");
        let found = found.unwrap();
        assert!(
            bundle::System
                .bundle_id(&found)
                .is_some_and(|id| id.starts_with("com.apple."))
        );
        for px in SIZES {
            let started = std::time::Instant::now();
            let png = render::icon_png(&found, px);
            assert!(png.is_some(), "{name} at {px}");
            let png = png.unwrap();
            assert!(super::cache::is_png(&png), "{name} at {px}");
            eprintln!(
                "{name} {px}px: {} bytes in {:?}",
                png.len(),
                started.elapsed()
            );
            if let Some(out) = &out {
                std::fs::create_dir_all(out).unwrap();
                std::fs::write(out.join(format!("{name}-{px}.png")), &png).unwrap();
            }
        }
    }
    // Terminal through the registry, as the rail asks for it.
    let ask = Ask {
        app_id: Some("terminal".into()),
        app_pid: None,
    };
    let url = icon_data_url(&cache, &ask, 64, &hosts, &bundle::System).unwrap();
    assert!(url.starts_with("data:image/png;base64,"));
    assert!(std::fs::read_dir(tmp.path().join("icons")).unwrap().count() == 1);
}
