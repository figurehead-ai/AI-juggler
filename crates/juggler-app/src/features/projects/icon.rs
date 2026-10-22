//! AJ-E1532: a project's own icon, found read-only in its folder: an app
//! icon (Tauri, Electron, Xcode), `public/favicon.*`, a `logo.*` or
//! `icon.*` at the root, or a logo under `.github/`. Only a regular file
//! (never a link) of at most [`MAX_ICON_BYTES`], whose first bytes say it
//! is a known image type, is taken; anything else is passed over for the
//! next place. The UI asks for an icon by project id only, and gets only
//! a file found here, as a `data:` URL it draws locally.

use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant, UNIX_EPOCH};

use base64::Engine;

use crate::error::AppResult;
use crate::state::AppState;

/// The largest icon read.
pub const MAX_ICON_BYTES: u64 = 512 * 1024;

/// How many entries of one folder are looked at, at most.
const MAX_ENTRIES: usize = 256;

/// A Tauri app's icons, in its `src-tauri/icons/` or (in a workspace)
/// `crates/<app>/icons/`.
const TAURI: &[&str] = &["128x128@2x.png", "icon.png", "128x128.png"];

/// An Electron app's icons.
const ELECTRON: &[&str] = &[
    "build/icon.png",
    "build/icons/512x512.png",
    "build/icons/256x256.png",
    "resources/icon.png",
    "assets/icon.png",
];

/// An icon found in a project's folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoundIcon {
    /// The file.
    pub path: PathBuf,
    /// Its version (size and modification time), so a changed icon is
    /// fetched again.
    pub version: String,
}

/// The image type `bytes` start like, as a media type.
#[must_use]
pub fn sniff(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Some("image/png");
    }
    if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        return Some("image/jpeg");
    }
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        return Some("image/gif");
    }
    if bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP" {
        return Some("image/webp");
    }
    if bytes.starts_with(&[0, 0, 1, 0]) && bytes.len() > 6 {
        return Some("image/x-icon");
    }
    let text = std::str::from_utf8(bytes).ok()?;
    let text = text.trim_start_matches('\u{feff}').trim_start();
    let lower = text.to_ascii_lowercase();
    let svg = lower.starts_with("<svg") || (lower.starts_with("<?xml") && lower.contains("<svg"));
    // Drawn as an image, an SVG runs nothing; one that carries scripts or
    // foreign content is passed over anyway.
    (svg && !lower.contains("<script") && !lower.contains("<foreignobject"))
        .then_some("image/svg+xml")
}

/// `path` as an icon: a regular file, not a link, at most
/// [`MAX_ICON_BYTES`], of a known image type.
fn accept(path: &Path) -> Option<FoundIcon> {
    let meta = std::fs::symlink_metadata(path).ok()?;
    if !meta.file_type().is_file() || meta.len() == 0 || meta.len() > MAX_ICON_BYTES {
        return None;
    }
    let mut head = Vec::with_capacity(512);
    std::fs::File::open(path)
        .ok()?
        .take(512)
        .read_to_end(&mut head)
        .ok()?;
    // A binary image is told by its head; an SVG only by all of it (its
    // head may stop inside a character, or before a script).
    match sniff(&head) {
        Some(mime) if mime != "image/svg+xml" => {}
        _ => {
            sniff(&std::fs::read(path).ok()?)?;
        }
    }
    let modified = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_secs());
    Some(FoundIcon {
        path: path.to_path_buf(),
        version: format!("{:x}-{modified:x}", meta.len()),
    })
}

/// A folder's entries (at most [`MAX_ENTRIES`]), sorted by name.
fn entries(dir: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .take(MAX_ENTRIES)
        .map(|entry| entry.path())
        .collect();
    out.sort();
    out
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

/// Files in `dir` whose name `wanted` accepts, SVG and PNG first.
fn named(dir: &Path, wanted: impl Fn(&str) -> bool) -> Vec<PathBuf> {
    let rank = |path: &PathBuf| match path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .as_deref()
    {
        Some("svg") => 0,
        Some("png") => 1,
        _ => 2,
    };
    let mut out: Vec<PathBuf> = entries(dir)
        .into_iter()
        .filter(|p| wanted(&file_name(p)))
        .collect();
    out.sort_by_key(rank);
    out
}

/// The places an icon may be, in the order they are tried.
fn candidates(root: &Path) -> Vec<PathBuf> {
    let tauri = std::iter::once(root.join("src-tauri")).chain(entries(&root.join("crates")));
    let mut out: Vec<PathBuf> = tauri
        .flat_map(|app| TAURI.iter().map(move |name| app.join("icons").join(name)))
        .collect();
    out.extend(ELECTRON.iter().map(|rel| root.join(rel)));
    // An Xcode app: its AppIcon set's largest image that fits.
    let mut xcode: Vec<PathBuf> = std::iter::once(root.to_path_buf())
        .chain(
            entries(root)
                .into_iter()
                .filter(|p| p.is_dir() && !file_name(p).starts_with('.')),
        )
        .flat_map(|dir| {
            named(
                &dir.join("Assets.xcassets").join("AppIcon.appiconset"),
                |n| {
                    Path::new(n)
                        .extension()
                        .is_some_and(|e| e.eq_ignore_ascii_case("png"))
                },
            )
        })
        .filter(|p| std::fs::metadata(p).is_ok_and(|m| m.len() <= MAX_ICON_BYTES))
        .collect();
    xcode.sort_by_key(|p| std::cmp::Reverse(std::fs::metadata(p).map_or(0, |m| m.len())));
    out.extend(xcode);
    out.extend(named(&root.join("public"), |n| n.starts_with("favicon.")));
    out.extend(named(root, |n| {
        n.starts_with("logo.") || n.starts_with("icon.")
    }));
    out.extend(named(&root.join(".github"), |n| n.contains("logo")));
    out
}

/// The project's own icon, when it has one.
#[must_use]
pub fn find_icon(root: &Path) -> Option<FoundIcon> {
    candidates(root).iter().find_map(|path| accept(path))
}

/// [`find_icon`], remembered for five minutes per folder.
#[must_use]
pub fn find_icon_cached(root: &Path) -> Option<FoundIcon> {
    type Seen = HashMap<PathBuf, (Option<FoundIcon>, Instant)>;
    static CACHE: OnceLock<Mutex<Seen>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(map) = cache.lock()
        && let Some((icon, at)) = map.get(root)
        && at.elapsed() < Duration::from_secs(300)
    {
        return icon.clone();
    }
    let icon = find_icon(root);
    if let Ok(mut map) = cache.lock() {
        map.insert(root.to_path_buf(), (icon.clone(), Instant::now()));
    }
    icon
}

/// Remembers the icon each project's mark uses (clones share one): the
/// only files [`projects_icon_read`] reads, per app state.
pub fn remember_served(state: &AppState, icons: impl IntoIterator<Item = (String, PathBuf)>) {
    if let Ok(mut map) = state.project_icons.lock() {
        *map = icons.into_iter().collect();
    }
}

/// The icon at `path` as a `data:` URL, checked again as it is read.
#[must_use]
pub fn data_url(path: &Path) -> Option<String> {
    accept(path)?;
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .ok()?
        .take(MAX_ICON_BYTES + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    if u64::try_from(bytes.len()).ok()? > MAX_ICON_BYTES {
        return None;
    }
    let mime = sniff(&bytes)?;
    Some(format!(
        "data:{mime};base64,{}",
        base64::engine::general_purpose::STANDARD.encode(&bytes)
    ))
}

/// A project's icon, as a `data:` URL; `None` when it has none.
///
/// # Errors
///
/// None today; the result type keeps the command's shape.
pub fn icon_read_impl(state: &Arc<AppState>, project_id: &str) -> AppResult<Option<String>> {
    let path = state
        .project_icons
        .lock()
        .ok()
        .and_then(|map| map.get(project_id).cloned());
    Ok(path.and_then(|path| data_url(&path)))
}

/// A project's own icon, for its mark.
///
/// # Errors
///
/// As [`icon_read_impl`].
#[tauri::command]
pub async fn projects_icon_read(
    state: tauri::State<'_, Arc<AppState>>,
    project_id: String,
) -> AppResult<Option<String>> {
    crate::ipc::blocking(state.inner(), "the project's icon", move |state| {
        icon_read_impl(state, &project_id)
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    const PNG: &[u8] = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR";
    const SVG: &str = "<svg xmlns=\"http://www.w3.org/2000/svg\"/>";

    fn write(root: &Path, rel: &str, bytes: &[u8]) -> PathBuf {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, bytes).unwrap();
        path
    }

    #[test]
    fn known_image_types_are_told_by_their_first_bytes() {
        assert_eq!(sniff(PNG), Some("image/png"));
        assert_eq!(sniff(&[0xff, 0xd8, 0xff, 0xe0]), Some("image/jpeg"));
        assert_eq!(sniff(b"GIF89a.."), Some("image/gif"));
        assert_eq!(sniff(b"RIFF\0\0\0\0WEBPVP8 "), Some("image/webp"));
        assert_eq!(sniff(&[0, 0, 1, 0, 1, 0, 16]), Some("image/x-icon"));
        assert_eq!(sniff(SVG.as_bytes()), Some("image/svg+xml"));
        assert_eq!(
            sniff(b"\xef\xbb\xbf<?xml version=\"1.0\"?>\n<svg/>"),
            Some("image/svg+xml")
        );
        assert_eq!(sniff(b"this is not an image\n"), None);
        assert_eq!(sniff(b"<svg><script>alert(1)</script></svg>"), None);
        assert_eq!(sniff(b"<svg><foreignObject/></svg>"), None);
        assert_eq!(sniff(b"<html></html>"), None);
    }

    #[test]
    fn each_kind_of_icon_is_found_and_app_icons_come_first() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        assert_eq!(find_icon(root), None);
        let github = write(root, ".github/docs-logo.svg", SVG.as_bytes());
        assert_eq!(find_icon(root).unwrap().path, github);
        let logo = write(root, "logo.png", PNG);
        assert_eq!(find_icon(root).unwrap().path, logo);
        let favicon = write(root, "public/favicon.ico", &[0, 0, 1, 0, 1, 0, 16, 16]);
        assert_eq!(find_icon(root).unwrap().path, favicon);
        write(
            root,
            "MyApp/Assets.xcassets/AppIcon.appiconset/small.png",
            PNG,
        );
        let large = write(
            root,
            "MyApp/Assets.xcassets/AppIcon.appiconset/large.png",
            &[PNG, &[0; 64]].concat(),
        );
        assert_eq!(find_icon(root).unwrap().path, large);
        let electron = write(root, "build/icon.png", PNG);
        assert_eq!(find_icon(root).unwrap().path, electron);
        let workspace = write(root, "crates/app/icons/icon.png", PNG);
        assert_eq!(find_icon(root).unwrap().path, workspace.clone());
        let tauri = write(root, "src-tauri/icons/icon.png", PNG);
        assert_eq!(find_icon(root).unwrap().path, tauri);
        assert!(find_icon(root).unwrap().version.contains('-'));
    }

    #[test]
    fn an_oversized_file_a_fake_image_and_a_link_are_passed_over() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let big = [PNG, &vec![0u8; usize::try_from(MAX_ICON_BYTES).unwrap()]].concat();
        write(root, "src-tauri/icons/icon.png", &big);
        write(root, "logo.png", b"this is not an image\n");
        let outside = tempfile::tempdir().unwrap();
        let target = write(outside.path(), "secret.png", PNG);
        std::fs::create_dir_all(root.join("public")).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&target, root.join("public/favicon.png")).unwrap();
        assert_eq!(find_icon(root), None);
        let fallback = write(root, ".github/logo.png", PNG);
        assert_eq!(find_icon(root).unwrap().path, fallback);
    }

    #[test]
    fn an_icon_is_served_as_a_data_url_only_once_the_list_found_it() {
        let tmp = tempfile::tempdir().unwrap();
        let path = write(tmp.path(), "logo.svg", SVG.as_bytes());
        let url = data_url(&path).unwrap();
        assert!(url.starts_with("data:image/svg+xml;base64,"));
        std::fs::write(&path, b"no longer an image").unwrap();
        assert_eq!(data_url(&path), None);
    }
}
