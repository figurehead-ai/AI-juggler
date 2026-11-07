//! AJ-E3508: each area's floor logo, drawn by the main window (the tool's
//! mark as `ToolMark` draws it, or a desktop app's own icon, with the
//! Account's initials in a ring of the App's colour) and kept here, one
//! PNG per App, in the registered local cache "Floor logos" (ADR-0023).
//!
//! - A file is named by the App and the logo's print (what it was drawn
//!   from), so a new mark, Account name or colour gets a new file and the
//!   old one goes; the snapshot says each App's print, so the window
//!   draws only what's missing.
//! - Nothing is bundled; a cleared cache is drawn again the next time the
//!   Floor shows.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use juggler_floor::paint::Img;
use juggler_floor::rng::hash;

use crate::error::{AppError, AppResult};
use crate::features::cache::{About, Gate, LocalCache, Trim, folder::FolderCache, stamp};
use crate::state::AppState;

/// The cache's key.
pub const CACHE: &str = "floor-logos";
/// The folder under AI Juggler's data folder.
pub const DIR: &str = "floor-logos";
/// The files' extension.
const EXTENSION: &str = "logo";
/// The files' format.
const FORMAT: u32 = 1;
/// The largest logo kept.
pub const MAX_PNG_BYTES: usize = 512 * 1024;
/// A logo's side, pixels.
pub const SIDE: u32 = 256;

/// The Floor logos, for the cache's registry.
#[must_use]
pub fn local_cache(data_dir: &Path) -> Arc<dyn LocalCache> {
    Arc::new(FolderCache::new(
        About {
            id: CACHE,
            name: "Floor logos",
            speeds_up: "Each area of the Floor shows its tool's logo at once.",
            slower: "The logos are drawn again the next time the Floor shows.",
            trim: Trim::Partly,
            own_switch: None,
            max_entries: Some(2000),
            shaped_by: &[],
        },
        data_dir.join(DIR),
        EXTENSION,
        &[],
        Box::new(|| (FORMAT, 0)),
    ))
}

/// The print the window computes for a logo, as hex (FNV-1a of its
/// words, the same as `floorLogoPrint` in the UI).
#[must_use]
pub fn print_hex(words: &str) -> String {
    format!("{:08x}", hash(words))
}

fn file_of(dir: &Path, app_id: &str, print: &str) -> PathBuf {
    dir.join(format!("{:08x}-{print}.{EXTENSION}", hash(app_id)))
}

fn valid_print(print: &str) -> bool {
    print.len() == 8 && print.bytes().all(|b| b.is_ascii_hexdigit())
}

/// The print of each App's kept logo, from the folder's names.
#[must_use]
pub fn prints(dir: &Path, apps: &[&str]) -> BTreeMap<String, String> {
    let by_hash: BTreeMap<String, &str> = apps
        .iter()
        .map(|id| (format!("{:08x}", hash(id)), *id))
        .collect();
    let mut out = BTreeMap::new();
    for entry in std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .take(4096)
    {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(stem) = name.strip_suffix(&format!(".{EXTENSION}")) else {
            continue;
        };
        let Some((app, print)) = stem.split_once('-') else {
            continue;
        };
        if let Some(id) = by_hash.get(app)
            && valid_print(print)
        {
            out.insert((*id).to_owned(), print.to_owned());
        }
    }
    out
}

fn stamp_for(print: &str) -> stamp::Stamp<'_> {
    stamp::Stamp {
        cache: CACHE,
        format: FORMAT,
        settings: 0,
        source: print,
    }
}

/// Keeps `png` as `app_id`'s logo drawn from `print`, and removes the
/// App's older ones.
///
/// # Errors
///
/// [`AppError::Refused`] for a print or picture that isn't one;
/// [`AppError::Io`] when it can't be written.
pub fn put(dir: &Path, gate: &Gate, app_id: &str, print: &str, png: &[u8]) -> AppResult<bool> {
    if !valid_print(print) || !png.starts_with(b"\x89PNG\r\n\x1a\n") || png.len() > MAX_PNG_BYTES {
        return Err(AppError::Refused("That isn't a floor logo.".into()));
    }
    let file = file_of(dir, app_id, print);
    let written = stamp::write(gate, &file, &stamp_for(print), png)
        .map_err(|e| AppError::io("floor logo", &e))?;
    let prefix = format!("{:08x}-", hash(app_id));
    let keep = file
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with(&prefix) && name != keep {
            let _ = std::fs::remove_file(entry.path());
        }
    }
    Ok(written)
}

/// `app_id`'s kept logo drawn from `print`, as pixels.
#[must_use]
pub fn read(dir: &Path, gate: &Gate, app_id: &str, print: &str) -> Option<Img> {
    let bytes = stamp::read(
        gate,
        &file_of(dir, app_id, print),
        &stamp_for(print),
        MAX_PNG_BYTES as u64 + 4096,
    )?;
    decode(&bytes)
}

/// A PNG as RGBA pixels (at most [`SIDE`] each way).
#[must_use]
pub fn decode(bytes: &[u8]) -> Option<Img> {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_transformations(
        png::Transformations::EXPAND | png::Transformations::STRIP_16 | png::Transformations::ALPHA,
    );
    let mut reader = decoder.read_info().ok()?;
    let mut buf = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut buf).ok()?;
    if info.width > SIDE || info.height > SIDE {
        return None;
    }
    let (w, h) = (
        i32::try_from(info.width).ok()?,
        i32::try_from(info.height).ok()?,
    );
    let mut img = Img::new(w, h);
    let px = &buf[..info.buffer_size()];
    match info.color_type {
        png::ColorType::Rgba => img.px.copy_from_slice(px),
        png::ColorType::GrayscaleAlpha => {
            for (dst, src) in img
                .px
                .as_chunks_mut::<4>()
                .0
                .iter_mut()
                .zip(px.as_chunks::<2>().0)
            {
                dst.copy_from_slice(&[src[0], src[0], src[0], src[1]]);
            }
        }
        _ => return None,
    }
    Some(img)
}

/// Keeps a logo the window drew (`floor_logo_put`); the Floor shows it
/// at its next rebuild.
///
/// # Errors
///
/// As [`put`].
pub fn put_impl(state: &Arc<AppState>, app_id: &str, print: &str, png: &[u8]) -> AppResult<()> {
    let dir = state.cache.data_dir().join(DIR);
    put(&dir, &state.cache.gate(), app_id, print, png)?;
    super::mark_stale(state);
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::{decode, local_cache, print_hex, prints, put, read};
    use crate::features::cache::Gate;

    fn png(w: u32, h: u32) -> Vec<u8> {
        let mut out = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut out, w, h);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().expect("header");
            writer
                .write_image_data(&vec![200; (w * h * 4) as usize])
                .expect("data");
        }
        out
    }

    #[test]
    fn a_logo_is_kept_by_its_print_and_an_older_one_goes() {
        let dir = tempfile::tempdir().expect("dir");
        let gate = Arc::new(Gate::default());
        let first = print_hex("tool|WM|1");
        assert!(put(dir.path(), &gate, "cli:work", &first, &png(4, 4)).expect("put"));
        assert_eq!(
            prints(dir.path(), &["cli:work"]).get("cli:work"),
            Some(&first)
        );
        let second = print_hex("tool|WM|2");
        put(dir.path(), &gate, "cli:work", &second, &png(4, 4)).expect("put");
        assert_eq!(
            prints(dir.path(), &["cli:work"]).get("cli:work"),
            Some(&second),
            "the colour changed: drawn again"
        );
        assert_eq!(
            std::fs::read_dir(dir.path()).expect("read").count(),
            1,
            "the old one went"
        );
        let img = read(dir.path(), &gate, "cli:work", &second).expect("read back");
        assert_eq!((img.w, img.h), (4, 4));
        assert!(read(dir.path(), &gate, "cli:work", &first).is_none());
    }

    #[test]
    fn only_a_real_logo_is_kept() {
        let dir = tempfile::tempdir().expect("dir");
        let gate = Arc::new(Gate::default());
        assert!(put(dir.path(), &gate, "a", "nothex!!", &png(4, 4)).is_err());
        assert!(put(dir.path(), &gate, "a", &print_hex("x"), b"not a png").is_err());
        assert!(decode(&png(300, 300)).is_none(), "too big");
        assert_eq!(print_hex("tool|WM|1").len(), 8);
    }

    /// AJ-E3508: the "Floor logos" cache clears, and the snapshot then
    /// says no logo is kept, so the window draws each one again.
    #[test]
    fn a_cleared_cache_leaves_every_logo_to_draw_again() {
        let data = tempfile::tempdir().expect("dir");
        let cache = local_cache(data.path());
        let gate = Arc::new(Gate::default());
        let print = print_hex("tool|WM|1");
        put(&cache.location(), &gate, "cli:work", &print, &png(4, 4)).expect("put");
        assert_eq!(cache.items(10).len(), 1, "the cache lists it");
        cache.clear().expect("clear");
        assert!(
            prints(&cache.location(), &["cli:work"]).is_empty(),
            "drawn again"
        );
        put(&cache.location(), &gate, "cli:work", &print, &png(4, 4)).expect("put");
        assert_eq!(
            prints(&cache.location(), &["cli:work"]).get("cli:work"),
            Some(&print),
            "and kept again"
        );
    }
}
