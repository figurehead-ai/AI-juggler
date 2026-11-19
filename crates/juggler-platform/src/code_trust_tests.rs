use std::io::Cursor;
use std::path::{Path, PathBuf};

use super::*;

const ID: &str = "ai.figurehead.juggler.test-helper";

#[test]
fn a_helper_is_looked_for_in_the_bundle_or_a_development_build_only() {
    let bundled = Path::new("/Applications/AI Juggler.app/Contents/MacOS/juggler-app");
    let beside = PathBuf::from("/Applications/AI Juggler.app/Contents/MacOS/helper");
    assert_eq!(helper_place(bundled, "helper", false), Some(beside.clone()));
    assert_eq!(helper_place(bundled, "helper", true), Some(beside));
    let built = Path::new("/work/target/debug/juggler-app");
    assert_eq!(
        helper_place(built, "helper", true),
        Some(PathBuf::from("/work/target/debug/helper")),
        "the shared target, in a development build"
    );
    assert_eq!(
        helper_place(built, "helper", false),
        None,
        "a release build outside a bundle runs no helper"
    );
    for lookalike in [
        "/tmp/Contents/MacOS/juggler-app",
        "/tmp/X.app/Contents/Other/juggler-app",
        "/tmp/X.app/MacOS/juggler-app",
        "/tmp/Xapp/Contents/MacOS/juggler-app",
        "juggler-app",
    ] {
        assert_eq!(
            helper_place(Path::new(lookalike), "helper", false),
            None,
            "{lookalike}"
        );
    }
}

#[test]
fn requirements_quote_only_plain_names() {
    assert_eq!(
        requirement(ID, &Signer::Local).as_deref(),
        Some(r#"identifier "ai.figurehead.juggler.test-helper""#)
    );
    assert_eq!(
        requirement(ID, &Signer::Team("ABCDE12345".to_owned())).as_deref(),
        Some(
            r#"anchor apple generic and certificate leaf[subject.OU] = "ABCDE12345" and identifier "ai.figurehead.juggler.test-helper""#
        )
    );
    for bad in ["", "a\" or anchor apple", "a b", "a)", "é"] {
        assert_eq!(requirement(bad, &Signer::Local), None, "{bad}");
        assert_eq!(
            requirement(ID, &Signer::Team(bad.to_owned())),
            None,
            "{bad}"
        );
    }
}

/// A 64-bit Mach-O slice for `cpu` whose code directory names `team`
/// (`None`: a directory from before teams).
fn slice(cpu: u32, team: Option<&str>) -> Vec<u8> {
    let le = |v: u32| v.to_le_bytes();
    let be = |v: u32| v.to_be_bytes();
    let signature_at = 256u32;
    // The code directory: 52 bytes of fields we read, then the team.
    let mut cd = Vec::new();
    let team_bytes = team
        .map(|t| [t.as_bytes(), b"\0"].concat())
        .unwrap_or_default();
    let cd_len = 52 + team_bytes.len() as u32;
    for v in [
        0xfade_0c02,
        cd_len,
        if team.is_some() { 0x20400 } else { 0x20100 },
    ] {
        cd.extend(be(v));
    }
    cd.resize(48, 0);
    cd.extend(be(if team.is_some() { 52 } else { 0 }));
    cd.extend(&team_bytes);
    // The superblob: requirements first, then the code directory.
    let mut blob = Vec::new();
    let cd_at = 12 + 2 * 8 + 12;
    for v in [0xfade_0cc0, cd_at + cd.len() as u32, 2, 2, 28, 0, cd_at] {
        blob.extend(be(v));
    }
    blob.extend([be(0xfade_0c01), be(12), be(0)].concat());
    blob.extend(&cd);
    // The header, a segment command, then the signature's.
    let mut out = Vec::new();
    for v in [0xfeed_facf, cpu, 0, 2, 2, 16 + 16, 0, 0] {
        out.extend(le(v));
    }
    for v in [0x19, 16, 0, 0] {
        out.extend(le(v));
    }
    for v in [0x1d, 16, signature_at, blob.len() as u32] {
        out.extend(le(v));
    }
    out.resize(signature_at as usize, 0);
    out.extend(blob);
    out
}

#[test]
fn the_team_is_read_from_the_code_directory() {
    let arm = 0x0100_000c;
    let intel = 0x0100_0007;
    assert_eq!(
        macho::team(&mut Cursor::new(slice(arm, Some("ABCDE12345")))).as_deref(),
        Some("ABCDE12345")
    );
    assert_eq!(macho::team(&mut Cursor::new(slice(arm, None))), None);
    assert_eq!(macho::team(&mut Cursor::new(b"#!/bin/sh\n".to_vec())), None);
    assert_eq!(macho::team(&mut Cursor::new(Vec::new())), None);
    // A universal file: this processor's slice.
    let (a, b) = (
        slice(intel, Some("INTEL00001")),
        slice(arm, Some("APPLE00001")),
    );
    let mut fat = Vec::new();
    for v in [0xcafe_babe_u32, 2, intel, 3, 4096, a.len() as u32, 12] {
        fat.extend(v.to_be_bytes());
    }
    for v in [arm, 0, 8192, b.len() as u32, 12] {
        fat.extend(v.to_be_bytes());
    }
    fat.resize(4096, 0);
    fat.extend(&a);
    fat.resize(8192, 0);
    fat.extend(&b);
    let want = if cfg!(target_arch = "x86_64") {
        "INTEL00001"
    } else {
        "APPLE00001"
    };
    assert_eq!(macho::team(&mut Cursor::new(fat)).as_deref(), Some(want));
    // Cut short anywhere: nothing, never a panic.
    let whole = slice(arm, Some("ABCDE12345"));
    for len in 0..whole.len() {
        let _ = macho::team(&mut Cursor::new(whole[..len].to_vec()));
    }
}

#[cfg(target_os = "macos")]
mod mac {
    use std::os::unix::fs::PermissionsExt;
    use std::process::Command;

    use super::*;

    fn codesign(args: &[&str], path: &Path) {
        let ok = Command::new("/usr/bin/codesign")
            .args(args)
            .arg(path)
            .output()
            .unwrap()
            .status
            .success();
        assert!(ok, "codesign {args:?}");
    }

    /// A copy of a system program, signed ad hoc as `identifier`.
    fn signed_copy(dir: &Path, name: &str, identifier: &str) -> PathBuf {
        let path = dir.join(name);
        std::fs::copy("/usr/bin/true", &path).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        codesign(
            &["--force", "--sign", "-", "--identifier", identifier],
            &path,
        );
        path
    }

    #[test]
    fn a_test_build_is_signed_without_a_team() {
        assert_eq!(own_signer(), Some(&Signer::Local));
        let exe = std::env::current_exe().unwrap();
        assert_eq!(
            macho::team(&mut std::fs::File::open(exe).unwrap()),
            None,
            "a linker or ad hoc signature names no team"
        );
    }

    #[test]
    fn only_a_valid_signature_with_the_right_identifier_and_team_passes() {
        let dir = tempfile::tempdir().unwrap();
        let good = signed_copy(dir.path(), "good", ID);
        assert!(signed_as(&good, ID, &Signer::Local));
        assert!(
            !signed_as(&good, "ai.figurehead.juggler.other", &Signer::Local),
            "another helper's identifier"
        );
        assert!(
            !signed_as(&good, ID, &Signer::Team("ABCDE12345".to_owned())),
            "not the app's team"
        );
        assert!(!signed_as(&dir.path().join("missing"), ID, &Signer::Local));

        // Tampered: one byte of its code changed after signing.
        let tampered = signed_copy(dir.path(), "tampered", ID);
        let mut bytes = std::fs::read(&tampered).unwrap();
        let base = macho::slice(&mut Cursor::new(&bytes)).unwrap() as usize;
        bytes[base + 0x1000] ^= 0xff;
        std::fs::write(&tampered, bytes).unwrap();
        assert!(!signed_as(&tampered, ID, &Signer::Local), "tampered");

        // Unsigned.
        let bare = signed_copy(dir.path(), "bare", ID);
        codesign(&["--remove-signature"], &bare);
        assert!(!signed_as(&bare, ID, &Signer::Local), "unsigned");

        // Not code at all.
        let script = dir.path().join("script");
        std::fs::write(&script, "#!/bin/sh\n").unwrap();
        assert!(!signed_as(&script, ID, &Signer::Local), "a script");
    }

    /// The resolver end to end, in this development build: a signed
    /// helper beside the executable is found; a missing one, one signed
    /// as something else or a tampered one is not.
    #[test]
    fn the_resolver_finds_only_a_trusted_signed_helper_beside_the_app() {
        assert_eq!(own_helper("juggler-no-such-helper", ID), None, "missing");
        let exe = std::fs::canonicalize(std::env::current_exe().unwrap()).unwrap();
        let dir = exe.parent().unwrap();
        let name = format!("juggler-code-trust-test-{}", std::process::id());
        let helper = signed_copy(dir, &name, ID);
        let me = crate::user::current().unwrap();
        let trusted = crate::file_trust::trusted_executable(&helper, me).is_some();
        let found = own_helper(&name, ID);
        let other = own_helper(&name, "ai.figurehead.juggler.other");
        let mut bytes = std::fs::read(&helper).unwrap();
        let base = macho::slice(&mut Cursor::new(&bytes)).unwrap() as usize;
        bytes[base + 0x1000] ^= 0xff;
        std::fs::write(&helper, bytes).unwrap();
        let tampered = own_helper(&name, ID);
        std::fs::remove_file(&helper).unwrap();
        // The build folder itself must be trusted for the first to hold.
        if trusted && cfg!(debug_assertions) {
            assert_eq!(found, Some(helper.clone()));
        }
        assert_eq!(other, None, "signed as another helper");
        assert_eq!(tampered, None, "tampered");
    }
}
