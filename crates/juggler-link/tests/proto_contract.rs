//! AJ-E703 protocol contract gates (they replace `buf lint` +
//! `buf breaking`, which would need a third-party CLI in CI):
//!
//! 1. **Breaking-change check** — every field recorded in the committed
//!    snapshot (`proto/link.v1.snapshot`) must still exist with the same
//!    number and type. Additions are fine; removal or renumbering fails.
//!    Regenerate the snapshot with `AIJ_BLESS=1` after an ADDITIVE change.
//! 2. **Payload allowlist** — no message field may be named like a
//!    credential, key, token, or transcript (the pack fetch, AJ-E706, is
//!    the one sanctioned exception and is not in v1).
//! 3. **Decode robustness** — random and mutated frames never panic the
//!    decoder (an in-CI stand-in for the 60 s fuzz run; the libFuzzer
//!    target lives in `fuzz/fuzz_targets/link_frame_decode.rs`).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::pedantic,
    missing_docs
)]

use std::path::PathBuf;

use prost::Message;

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `package.Message.field = number:type` lines, sorted.
fn current_fields() -> Vec<String> {
    let pool =
        protox::compile(["juggler/link/v1/link.proto"], [crate_dir().join("proto")]).unwrap();
    let mut lines = Vec::new();
    for file in &pool.file {
        let package = file.package();
        for message in &file.message_type {
            for field in &message.field {
                lines.push(format!(
                    "{package}.{}.{} = {}:{:?}{}",
                    message.name(),
                    field.name(),
                    field.number(),
                    field.r#type(),
                    if field.type_name().is_empty() {
                        String::new()
                    } else {
                        format!(" {}", field.type_name())
                    }
                ));
            }
        }
    }
    lines.sort();
    lines
}

#[test]
fn no_field_was_removed_or_renumbered() {
    let path = crate_dir().join("proto/link.v1.snapshot");
    let current = current_fields();
    if std::env::var_os("AIJ_BLESS").is_some() {
        std::fs::write(&path, current.join("\n") + "\n").unwrap();
    }
    let committed = std::fs::read_to_string(&path).expect("the snapshot is committed");
    for line in committed.lines().filter(|line| !line.trim().is_empty()) {
        assert!(
            current.iter().any(|now| now == line),
            "breaking change: `{line}` was removed, renumbered or retyped — reserve the field instead"
        );
    }
}

#[test]
fn no_field_can_carry_a_secret_or_a_transcript() {
    let forbidden = [
        "token",
        "password",
        "secret",
        "credential",
        "cookie",
        "private",
        "keychain",
        "transcript",
        "auth",
        "bearer",
        "api_key",
        "seed",
        // AJ-E706: a path never crosses the link (project roots travel
        // as digests; the one transcript-bearing message is the
        // user-clicked handover bundle, whose files have flat names).
        "path",
        "cwd",
        "dir",
    ];
    for line in current_fields() {
        let field = line
            .split(" = ")
            .next()
            .unwrap()
            .rsplit('.')
            .next()
            .unwrap()
            .to_ascii_lowercase();
        for word in forbidden {
            assert!(
                !field.contains(word),
                "`{line}` looks like it could carry `{word}`"
            );
        }
    }
}

#[test]
fn random_and_mutated_frames_never_panic_the_decoder() {
    // A deterministic xorshift so failures reproduce.
    let mut state = 0x9E37_79B9_7F4A_7C15_u64;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    let valid = juggler_link::proto::ControlCommand {
        console_machine_id: "console-b".into(),
        nonce: vec![1; 16],
        sequence: 7,
        issued_at_ms: 1,
        action: Some(juggler_link::proto::control_command::Action::StopInstance(
            juggler_link::proto::StopInstance {
                instance_id: "inst-1".into(),
            },
        )),
    }
    .encode_to_vec();
    for round in 0..20_000 {
        let frame: Vec<u8> = if round % 2 == 0 {
            let len = (next() % 512) as usize;
            (0..len).map(|_| next() as u8).collect()
        } else {
            let mut mutated = valid.clone();
            let flips = 1 + (next() % 4) as usize;
            for _ in 0..flips {
                let at = (next() as usize) % mutated.len();
                mutated[at] ^= next() as u8;
            }
            mutated
        };
        let _ = juggler_link::proto::ControlCommand::decode(frame.as_slice());
        let _ = juggler_link::proto::TelemetryDelta::decode(frame.as_slice());
        let _ = juggler_link::proto::HelloRequest::decode(frame.as_slice());
    }
}
