//! AJ-E1010 — the real updater plugin (`tauri-plugin-updater`, the code
//! release builds run) against a local feed. No fake: requests are
//! counted at the socket, and packages are signed in the test with
//! minisign's format, so the plugin's own signature check is what
//! refuses them.
//!
//! - With "check when AI Juggler starts" off, the launch check opens
//!   **zero** connections; turned on, the same call reaches the feed (so
//!   the zero is measured, not assumed).
//! - A package that fails its signature — changed after signing, signed
//!   with another key, or signed for another version — never installs:
//!   the throwaway app bundle it would replace stays byte for byte as it
//!   was, and the user reads why in plain words.
//! - A network failure mid-download leaves it untouched as well.
//! - The control: the same feed with a good signature installs, and the
//!   bundle is replaced. Nothing restarts.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use base64::Engine as _;
use ed25519_dalek::{Signer as _, SigningKey};
use juggler_adapter::AdapterRegistry;
use juggler_app::state::AppState;
use juggler_app::updates::{self, PluginFeed, UpdateManager};
use tauri::test::MockRuntime;

const B64: base64::engine::GeneralPurpose = base64::engine::general_purpose::STANDARD;
const KEY_ID: [u8; 8] = *b"AIJ-TEST";
const OFFERED: &str = "9.9.9";

// --- a minimal HTTP feed that counts every connection ---------------------

type Routes = Arc<Mutex<HashMap<String, (u16, Vec<u8>)>>>;

struct Feed {
    base: String,
    connections: Arc<AtomicUsize>,
    routes: Routes,
}

impl Feed {
    fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let connections = Arc::new(AtomicUsize::new(0));
        let routes: Routes = Arc::default();
        let (count, table) = (Arc::clone(&connections), Arc::clone(&routes));
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                count.fetch_add(1, Ordering::SeqCst);
                let mut request = Vec::new();
                let mut buf = [0u8; 1024];
                while !request.windows(4).any(|w| w == b"\r\n\r\n") {
                    match stream.read(&mut buf) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => request.extend_from_slice(&buf[..n]),
                    }
                }
                let text = String::from_utf8_lossy(&request);
                let path = text.split_whitespace().nth(1).unwrap_or("/").to_owned();
                let (code, body) = table
                    .lock()
                    .unwrap()
                    .get(&path)
                    .cloned()
                    .unwrap_or((404, b"not found".to_vec()));
                if code == 0 {
                    // A dropped connection: headers promise more than comes.
                    let _ = stream.write_all(
                        format!(
                            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            body.len() * 4
                        )
                        .as_bytes(),
                    );
                    let _ = stream.write_all(&body);
                    continue;
                }
                let _ = stream.write_all(
                    format!(
                        "HTTP/1.1 {code} X\r\nContent-Length: {}\r\nContent-Type: application/octet-stream\r\nConnection: close\r\n\r\n",
                        body.len()
                    )
                    .as_bytes(),
                );
                // In pieces, as a real download arrives.
                for piece in body.chunks(32 * 1024) {
                    if stream.write_all(piece).is_err() {
                        break;
                    }
                    let _ = stream.flush();
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
            }
        });
        Self {
            base,
            connections,
            routes,
        }
    }

    fn serve(&self, path: &str, code: u16, body: Vec<u8>) {
        self.routes
            .lock()
            .unwrap()
            .insert(path.to_owned(), (code, body));
    }

    fn connections(&self) -> usize {
        self.connections.load(Ordering::SeqCst)
    }

    /// `latest.json` announcing [`OFFERED`] with `signature` for the
    /// package at `/pkg.tar.gz`.
    fn announce(&self, signature: &str) {
        let platform = serde_json::json!({
            "signature": signature,
            "url": format!("{}/pkg.tar.gz", self.base),
        });
        let latest = serde_json::json!({
            "version": OFFERED,
            "notes": "Plain notes for this version.\n\n## Fixed\n- Something users noticed.",
            "pub_date": "2026-10-20T08:00:00Z",
            // Every platform CI runs on, so the check finds the update.
            "platforms": {
                "darwin-aarch64": platform,
                "darwin-x86_64": platform,
                "linux-x86_64": platform,
                "linux-aarch64": platform,
                "windows-x86_64": platform,
            },
        });
        self.serve("/latest.json", 200, serde_json::to_vec(&latest).unwrap());
    }
}

// --- minisign, as `tauri signer sign` writes it ------------------------------

fn key(seed: u8) -> SigningKey {
    SigningKey::from_bytes(&[seed; 32])
}

/// The public key as the app carries it (base64 of the minisign file).
fn pubkey(key: &SigningKey) -> String {
    let mut bin = b"Ed".to_vec();
    bin.extend_from_slice(&KEY_ID);
    bin.extend_from_slice(&key.verifying_key().to_bytes());
    B64.encode(format!(
        "untrusted comment: minisign public key\n{}\n",
        B64.encode(bin)
    ))
}

/// The release signature of `data`, signed for `version`, as the feed
/// carries it (base64 of the minisign `.sig` file).
fn sign(key: &SigningKey, data: &[u8], version: &str) -> String {
    sign_trusted(
        key,
        data,
        &format!("timestamp:1792483200\tfile:pkg.tar.gz\tversion:{version}"),
    )
}

/// The same, with any trusted comment: `tauri signer sign` without
/// `--app-version` writes none with a version (0.1.0's packages).
fn sign_trusted(key: &SigningKey, data: &[u8], trusted: &str) -> String {
    let signature = key.sign(data).to_bytes();
    let mut bin = b"Ed".to_vec();
    bin.extend_from_slice(&KEY_ID);
    bin.extend_from_slice(&signature);
    let mut global = signature.to_vec();
    global.extend_from_slice(trusted.as_bytes());
    let global = key.sign(&global).to_bytes();
    B64.encode(format!(
        "untrusted comment: signature from tauri secret key\n{}\ntrusted comment: {trusted}\n{}\n",
        B64.encode(bin),
        B64.encode(global)
    ))
}

// --- the app being updated ----------------------------------------------------

/// A throwaway `Fixture.app` whose executable the updater would replace.
fn installed_bundle(root: &Path) -> PathBuf {
    let exe = root.join("Fixture.app/Contents/MacOS/fixture");
    std::fs::create_dir_all(exe.parent().unwrap()).unwrap();
    std::fs::write(&exe, b"the running version").unwrap();
    std::fs::write(
        root.join("Fixture.app/Contents/Info.plist"),
        b"<plist>running</plist>",
    )
    .unwrap();
    exe
}

/// The update package: `Fixture.app` as a gzipped tar, like the release.
fn package(contents: &[u8]) -> Vec<u8> {
    let mut tar = tar::Builder::new(flate2::write::GzEncoder::new(
        Vec::new(),
        flate2::Compression::default(),
    ));
    for (path, bytes) in [
        ("Fixture.app/Contents/MacOS/fixture", contents),
        (
            "Fixture.app/Contents/Info.plist",
            b"<plist>new</plist>".as_slice(),
        ),
    ] {
        let mut header = tar::Header::new_gnu();
        header.set_size(bytes.len() as u64);
        header.set_mode(0o755);
        header.set_cksum();
        tar.append_data(&mut header, path, bytes).unwrap();
    }
    tar.into_inner().unwrap().finish().unwrap()
}

fn mock_app() -> tauri::App<MockRuntime> {
    let mut context = tauri::test::mock_context(tauri::test::noop_assets());
    context.config_mut().plugins.0.insert(
        "updater".into(),
        serde_json::json!({
            "pubkey": "",
            "endpoints": [],
            "requireSignedVersion": true,
            "dangerousInsecureTransportProtocol": true,
        }),
    );
    tauri::test::mock_builder()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .build(context)
        .unwrap()
}

fn app_state(dir: &Path) -> Arc<AppState> {
    let mut state =
        AppState::init_with_registry(dir.to_path_buf(), AdapterRegistry::new()).unwrap();
    state.updates = UpdateManager::for_test(true);
    Arc::new(state)
}

fn snapshot(root: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut files = Vec::new();
    let mut stack = vec![root.join("Fixture.app")];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else {
                files.push((path.clone(), std::fs::read(&path).unwrap()));
            }
        }
    }
    files.sort();
    files
}

#[test]
fn check_off_means_zero_connections_and_on_reaches_the_feed() {
    let work = tempfile::tempdir().unwrap();
    let feed = Feed::start();
    let good = key(7);
    feed.announce(&sign(&good, b"unused", OFFERED));
    let app = mock_app();
    let exe = installed_bundle(work.path());
    let plugin = PluginFeed::pinned(
        app.handle().clone(),
        &pubkey(&good),
        &format!("{}/latest.json", feed.base),
        Some(exe),
    );
    let state = app_state(&work.path().join("data"));

    updates::update_set_check_on_launch_impl(&state, false).unwrap();
    let asked = updates::launch_check(&state, &plugin);
    std::thread::sleep(std::time::Duration::from_millis(200));
    assert_eq!(
        feed.connections(),
        0,
        "turned off: the launch check opens no connection at all"
    );
    assert!(!asked);

    // The control: on, the very same call reaches the feed.
    updates::update_set_check_on_launch_impl(&state, true).unwrap();
    assert!(updates::launch_check(&state, &plugin));
    assert_eq!(feed.connections(), 1);
    let status = updates::update_status_impl(&state);
    assert_eq!(status.state, "available", "{status:?}");
    let offered = status.available.unwrap();
    assert_eq!(offered.version, OFFERED);
    assert!(offered.notes.contains("Something users noticed"));
    assert_eq!(
        feed.connections(),
        1,
        "a check only reads the feed: nothing is downloaded"
    );
}

#[test]
fn a_package_that_fails_its_signature_never_installs() {
    let work = tempfile::tempdir().unwrap();
    let feed = Feed::start();
    let release_key = key(7);
    let app = mock_app();
    let exe = installed_bundle(work.path());
    let before = snapshot(work.path());
    let plugin = PluginFeed::pinned(
        app.handle().clone(),
        &pubkey(&release_key),
        &format!("{}/latest.json", feed.base),
        Some(exe),
    );
    let state = app_state(&work.path().join("data"));
    // A few megabytes that don't compress, so the download arrives in
    // many pieces (progress) like a real one.
    let mut seed = 0x2545_f491_u32;
    let mut new_app = b"the new version ".to_vec();
    new_app.extend((0..4 << 20).map(|_| {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        seed.to_le_bytes()[0]
    }));
    let new_version = package(&new_app);

    let cases: [(&str, Vec<u8>, String, &str, &str); 5] = [
        (
            "changed after signing",
            {
                let mut changed = new_version.clone();
                let last = changed.len() - 1;
                changed[last / 2] ^= 0xff;
                changed
            },
            sign(&release_key, &new_version, OFFERED),
            "signature",
            "didn't pass the signature check",
        ),
        (
            "signed with another key",
            new_version.clone(),
            sign(&key(9), &new_version, OFFERED),
            "signature",
            "didn't pass the signature check",
        ),
        (
            "signed for another version (a downgrade)",
            new_version.clone(),
            sign(&release_key, &new_version, "0.0.1"),
            "wrong_version",
            "signed for a different version",
        ),
        (
            // 0.1.0's packages: 0.0.12 refused them (2026-10-24), which is
            // why the release check now requires the version.
            "signed without its version",
            new_version.clone(),
            sign_trusted(
                &release_key,
                &new_version,
                "timestamp:1792483200\tfile:pkg.tar.gz",
            ),
            "signature",
            "didn't pass the signature check",
        ),
        (
            "not a signature at all",
            new_version.clone(),
            B64.encode("garbage"),
            "signature",
            "didn't pass the signature check",
        ),
    ];
    for (case, served, signature, kind, words) in cases {
        feed.announce(&signature);
        feed.serve("/pkg.tar.gz", 200, served);
        let checked = updates::update_check_impl(&state, &plugin).unwrap();
        assert_eq!(checked.state, "available", "{case}: {checked:?}");
        let status = updates::update_install_impl(&state, &plugin, &mut |_| {}).unwrap();
        assert_eq!(status.state, "failed", "{case}: {status:?}");
        assert_eq!(
            status.error_kind.as_deref(),
            Some(kind),
            "{case}: {status:?}"
        );
        let error = status.error.unwrap();
        assert!(error.contains(words), "{case}: {error}");
        assert!(error.contains("untouched"), "{case}: {error}");
        assert_eq!(
            snapshot(work.path()),
            before,
            "{case}: the installed app is byte for byte what it was"
        );
    }

    // A download that breaks off is the network's fault, and also
    // installs nothing.
    feed.announce(&sign(&release_key, &new_version, OFFERED));
    feed.serve(
        "/pkg.tar.gz",
        0,
        new_version[..new_version.len() / 2].to_vec(),
    );
    updates::update_check_impl(&state, &plugin).unwrap();
    let status = updates::update_install_impl(&state, &plugin, &mut |_| {}).unwrap();
    assert_eq!(status.state, "failed", "{status:?}");
    assert!(
        matches!(status.error_kind.as_deref(), Some("network" | "signature")),
        "{status:?}"
    );
    assert_eq!(snapshot(work.path()), before);

    // The control: correctly signed, it verifies — and on macOS, where the
    // package is an app bundle, it installs: the bundle is replaced.
    // Nothing restarts.
    feed.serve("/pkg.tar.gz", 200, new_version.clone());
    updates::update_check_impl(&state, &plugin).unwrap();
    #[cfg(not(target_os = "macos"))]
    {
        use juggler_app::updates::UpdateFeed as _;
        let mut progress = Vec::new();
        let bytes = plugin
            .download(&mut |chunk, total| progress.push((chunk, total)))
            .expect("a correctly signed package verifies");
        assert_eq!(bytes, new_version);
        assert!(progress.len() > 20, "it arrives in pieces");
    }
    #[cfg(target_os = "macos")]
    {
        let mut progress = Vec::new();
        let status =
            updates::update_install_impl(&state, &plugin, &mut |s| progress.extend(s.progress))
                .unwrap();
        assert_eq!(status.state, "installed", "{status:?}");
        assert_eq!(
            std::fs::read(work.path().join("Fixture.app/Contents/MacOS/fixture")).unwrap(),
            new_app,
            "the new version is in place"
        );
        assert_eq!(progress.last(), Some(&100));
        // Every rise the download brings is shown, in order. How big each
        // step is depends on how the bytes arrive: the feed sends 32 KB
        // pieces (under 1 % of the package), but a loaded machine can read
        // several at once (a step over 5 % once). So: always rising, and many
        // steps, never one jump from 0 to 100.
        assert!(
            progress.windows(2).all(|w| w[1] > w[0]),
            "progress only rises: {progress:?}"
        );
        assert!(progress.len() >= 20, "progress in many steps: {progress:?}");
    }
}
