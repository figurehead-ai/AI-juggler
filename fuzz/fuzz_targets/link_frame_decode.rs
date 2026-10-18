#![no_main]
//! Fuzz target: Fleet link frames (AJ-E703). Decoding any bytes as any
//! link message must never panic — the node decodes peer input before
//! it can authorise it. Run: `cargo +nightly fuzz run link_frame_decode`
//! from `fuzz/`.

use libfuzzer_sys::fuzz_target;
use prost::Message;

fuzz_target!(|data: &[u8]| {
    let _ = juggler_link::proto::ControlCommand::decode(data);
    let _ = juggler_link::proto::TelemetryDelta::decode(data);
    let _ = juggler_link::proto::HelloRequest::decode(data);
    let _ = juggler_link::proto::Ping::decode(data);
});
