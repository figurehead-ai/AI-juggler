//! AI Juggler's own helper programs, and whether one may run: the one
//! resolver for a helper that ships inside the app (AJ-E3802, ADR-0033
//! §4.1).
//!
//! - **Where it may be:** beside the app's executable in its own bundle
//!   (`<App>.app/Contents/MacOS/<name>`), or, in a development build only,
//!   beside the executable in the build folder (the shared target). A
//!   release build outside a bundle has no helpers. Nothing else is
//!   looked at: no `PATH`, no other folder.
//! - **Who may own it:** [`crate::file_trust::trusted_executable`] (ours
//!   or root's, writable by no one else, and the folders above it too).
//! - **Who signed it:** its code signature is valid (strictly, every
//!   architecture), names the helper's own identifier, and, when the app
//!   is signed by a developer team, is that same team's
//!   ([`Signer::Team`]). An app without a team (a development build, the
//!   unsigned release candidate) runs only a validly signed helper with
//!   the right identifier ([`Signer::Local`]): whoever can change such a
//!   helper can change the app beside it.
//!
//! Anything that fails is not run, and the caller treats the helper as
//! missing (fails closed). Windows and Linux have no helpers yet: nothing
//! is found there.

use std::path::{Path, PathBuf};

/// Who signed AI Juggler itself, as far as its helpers go.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Signer {
    /// A developer team's certificate (Apple's Developer ID): a helper
    /// must be signed by the same team.
    Team(String),
    /// No team: unsigned, signed ad hoc, or by a certificate Apple didn't
    /// issue. A helper must still carry a valid signature.
    Local,
}

/// Where the helper `name` would be for the executable `exe`: in its
/// bundle's `Contents/MacOS`, or beside it in a development build (`dev`);
/// `None` for a release build outside a bundle.
#[must_use]
pub fn helper_place(exe: &Path, name: &str, dev: bool) -> Option<PathBuf> {
    let dir = exe.parent()?;
    let contents = dir.parent()?;
    let in_bundle = dir.file_name().is_some_and(|n| n == "MacOS")
        && contents.file_name().is_some_and(|n| n == "Contents")
        && contents
            .parent()
            .and_then(Path::extension)
            .is_some_and(|ext| ext == "app");
    (in_bundle || dev).then(|| dir.join(name))
}

/// The helper `name`, signed as `identifier`, for this running app, when
/// it may run (see the module's rules); its resolved path.
#[must_use]
pub fn own_helper(name: &str, identifier: &str) -> Option<PathBuf> {
    let exe = std::fs::canonicalize(std::env::current_exe().ok()?).ok()?;
    let place = helper_place(&exe, name, cfg!(debug_assertions))?;
    let me = crate::user::current().ok()?;
    let path = crate::file_trust::trusted_executable(&place, me)?;
    signed_as(&path, identifier, own_signer()?).then_some(path)
}

/// A name a code requirement may quote: letters, digits, `.` and `-`.
fn quotable(text: &str) -> bool {
    !text.is_empty()
        && text
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'-')
}

/// The part of a code requirement that names a developer team (issued
/// by Apple); `None` for a team that can't be quoted.
fn team_requirement(team: &str) -> Option<String> {
    quotable(team)
        .then(|| format!("anchor apple generic and certificate leaf[subject.OU] = \"{team}\""))
}

/// The code requirement a helper signed as `identifier` must meet for an
/// app signed by `signer`; `None` for a name that can't be quoted.
#[must_use]
pub fn requirement(identifier: &str, signer: &Signer) -> Option<String> {
    let identifier = quotable(identifier).then(|| format!("identifier \"{identifier}\""))?;
    match signer {
        Signer::Team(team) => Some(format!("{} and {identifier}", team_requirement(team)?)),
        Signer::Local => Some(identifier),
    }
}

/// Who signed this running app; `None` when it can't be told (then no
/// helper runs). Read once.
#[must_use]
pub fn own_signer() -> Option<&'static Signer> {
    static SIGNER: std::sync::OnceLock<Option<Signer>> = std::sync::OnceLock::new();
    SIGNER.get_or_init(mac::own_signer).as_ref()
}

/// Whether the code at `path` is validly signed as `identifier`, by the
/// team `signer` names (see [`requirement`]).
#[must_use]
pub fn signed_as(path: &Path, identifier: &str, signer: &Signer) -> bool {
    requirement(identifier, signer).is_some_and(|text| mac::meets(path, &text))
}

#[cfg(target_os = "macos")]
mod mac {
    use std::path::Path;

    use core_foundation::url::CFURL;
    use security_framework::os::macos::code_signing::{
        Flags, SecCode, SecRequirement, SecStaticCode,
    };

    use super::Signer;

    /// Strict, every architecture, and no network (no revocation look-up
    /// stalls a probe).
    fn strict() -> Flags {
        Flags::STRICT_VALIDATE | Flags::CHECK_ALL_ARCHITECTURES | Flags::NO_NETWORK_ACCESS
    }

    pub(super) fn meets(path: &Path, requirement: &str) -> bool {
        let Ok(requirement) = requirement.parse::<SecRequirement>() else {
            return false;
        };
        let Some(url) = CFURL::from_path(path, false) else {
            return false;
        };
        SecStaticCode::from_path(&url, Flags::NONE)
            .and_then(|code| code.check_validity(strict(), &requirement))
            .is_ok()
    }

    /// Apple-anchored: its team read from its own signature, and the
    /// running code checked against that team (so a misread can only
    /// fail). Otherwise [`Signer::Local`].
    pub(super) fn own_signer() -> Option<Signer> {
        let me = SecCode::for_self(Flags::NONE).ok()?;
        let apple = "anchor apple generic".parse::<SecRequirement>().ok()?;
        if me.check_validity(Flags::NONE, &apple).is_err() {
            return Some(Signer::Local);
        }
        let exe = std::env::current_exe().ok()?;
        let team = super::macho::team(&mut std::fs::File::open(exe).ok()?)?;
        let ours = super::team_requirement(&team)?
            .parse::<SecRequirement>()
            .ok()?;
        me.check_validity(Flags::NONE, &ours).ok()?;
        Some(Signer::Team(team))
    }
}

#[cfg(not(target_os = "macos"))]
mod mac {
    use std::path::Path;

    use super::Signer;

    pub(super) fn meets(_path: &Path, _requirement: &str) -> bool {
        false
    }

    pub(super) fn own_signer() -> Option<Signer> {
        None
    }
}

/// The team identifier in a Mach-O executable's code signature (its
/// code directory), read without running anything. Only the slice for
/// this processor (or a thin file) is read.
#[cfg(any(target_os = "macos", test))]
mod macho {
    use std::io::{Read, Seek, SeekFrom};

    const FAT_MAGIC: u32 = 0xcafe_babe;
    const FAT_MAGIC_64: u32 = 0xcafe_babf;
    const MH_MAGIC_64: u32 = 0xfeed_facf;
    const LC_CODE_SIGNATURE: u32 = 0x1d;
    const SUPERBLOB: u32 = 0xfade_0cc0;
    const CODE_DIRECTORY: u32 = 0xfade_0c02;
    /// The first code directory version with a team.
    const WITH_TEAM: u32 = 0x0002_0200;
    const CPU_ARM64: u32 = 0x0100_000c;
    const CPU_X86_64: u32 = 0x0100_0007;
    /// No signature here is bigger (a guard against a corrupt length).
    const MAX_BLOB: u64 = 16 * 1024 * 1024;

    fn bytes(file: &mut (impl Read + Seek), at: u64, len: u64) -> Option<Vec<u8>> {
        if len > MAX_BLOB {
            return None;
        }
        file.seek(SeekFrom::Start(at)).ok()?;
        let mut out = vec![0; usize::try_from(len).ok()?];
        file.read_exact(&mut out).ok()?;
        Some(out)
    }

    fn be(data: &[u8], at: usize) -> Option<u32> {
        Some(u32::from_be_bytes(data.get(at..at + 4)?.try_into().ok()?))
    }

    fn le(data: &[u8], at: usize) -> Option<u32> {
        Some(u32::from_le_bytes(data.get(at..at + 4)?.try_into().ok()?))
    }

    fn be64(data: &[u8], at: usize) -> Option<u64> {
        Some(u64::from_be_bytes(data.get(at..at + 8)?.try_into().ok()?))
    }

    /// Where this processor's slice starts.
    pub(super) fn slice(file: &mut (impl Read + Seek)) -> Option<u64> {
        let head = bytes(file, 0, 8)?;
        let magic = be(&head, 0)?;
        if magic != FAT_MAGIC && magic != FAT_MAGIC_64 {
            return Some(0);
        }
        let count = u64::from(be(&head, 4)?.min(64));
        let entry: u64 = if magic == FAT_MAGIC { 20 } else { 32 };
        let table = bytes(file, 8, count * entry)?;
        let want = if cfg!(target_arch = "x86_64") {
            CPU_X86_64
        } else {
            CPU_ARM64
        };
        let mut first = None;
        for i in 0..usize::try_from(count).ok()? {
            let at = i * usize::try_from(entry).ok()?;
            let offset = if magic == FAT_MAGIC {
                u64::from(be(&table, at + 8)?)
            } else {
                be64(&table, at + 8)?
            };
            if be(&table, at)? == want {
                return Some(offset);
            }
            first.get_or_insert(offset);
        }
        first
    }

    /// The team in the signature's code directory; `None` when there is
    /// none, or the file can't be read as signed 64-bit Mach-O.
    pub(super) fn team(file: &mut (impl Read + Seek)) -> Option<String> {
        let base = slice(file)?;
        let header = bytes(file, base, 32)?;
        if le(&header, 0)? != MH_MAGIC_64 {
            return None;
        }
        let commands = bytes(file, base + 32, u64::from(le(&header, 20)?))?;
        let mut at = 0;
        let mut signature = None;
        for _ in 0..le(&header, 16)? {
            let (cmd, size) = (le(&commands, at)?, le(&commands, at + 4)? as usize);
            if cmd == LC_CODE_SIGNATURE {
                signature = Some((le(&commands, at + 8)?, le(&commands, at + 12)?));
                break;
            }
            if size < 8 {
                return None;
            }
            at += size;
        }
        let (offset, size) = signature?;
        let blob = bytes(file, base + u64::from(offset), u64::from(size))?;
        if be(&blob, 0)? != SUPERBLOB {
            return None;
        }
        let directory = (0..be(&blob, 8)? as usize).find_map(|i| {
            let at = 12 + i * 8;
            (be(&blob, at)? == 0).then(|| be(&blob, at + 4))?
        })? as usize;
        let cd = blob.get(directory..)?;
        if be(cd, 0)? != CODE_DIRECTORY || be(cd, 8)? < WITH_TEAM {
            return None;
        }
        let team_at = be(cd, 48)? as usize;
        if team_at == 0 {
            return None;
        }
        let rest = cd.get(team_at..be(cd, 4)? as usize)?;
        let end = rest.iter().position(|b| *b == 0)?;
        String::from_utf8(rest[..end].to_vec()).ok()
    }
}

#[cfg(test)]
#[path = "code_trust_tests.rs"]
mod tests;
