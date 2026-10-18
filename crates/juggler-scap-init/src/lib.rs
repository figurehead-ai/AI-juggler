//! Vendored, audited ScreenCaptureKit init shim (decision D-17,
//! 2026-10-13 — the maintainer's screen-capture permission).
//!
//! THE AUDIT (the whole point of this crate): the `screencapturekit`
//! binding requires exactly one call to its own public FFI export
//! `sc_initialize_core_graphics()` before the first
//! `SCShareableContent` use, or the process SIGSEGVs inside
//! CoreGraphics on first capture (reproduced and logged at
//! `docs/qa/evidence/AJ-E415/real-capture-crash.log`). The call
//! forces CoreGraphics initialization (the bridge implements it via
//! `CGMainDisplayID`) and is idempotent.
//!
//! What this crate is NOT: a general unsafe escape hatch. It exists
//! so the workspace law `unsafe_code = "forbid"` stays absolute for
//! every other crate — this one deliberately does not inherit the
//! workspace lint baseline (see `Cargo.toml`) and contains ONE
//! `unsafe` block, zero logic, zero data, zero dependencies beyond
//! the binding itself. Any additional `unsafe` anywhere requires a
//! new recorded decision.
#![cfg(target_os = "macos")]
#![deny(missing_docs)]
#![allow(unsafe_code)] // D-17: the single audited call this crate exists for

/// Runs the audited initializer exactly once per process.
pub fn initialize() {
    INIT.call_once(|| {
        // SAFETY (D-17 audit): `sc_initialize_core_graphics` is the
        // binding's own exported initializer — idempotent, no
        // arguments, no return, no pointers, no re-entrancy; calling
        // it before any ScreenCaptureKit API is the vendor-documented
        // requirement. There is no soundness surface beyond "call it
        // once, early", which is what this `Once` enforces.
        unsafe {
            screencapturekit::ffi::sc_initialize_core_graphics();
        }
    });
}

static INIT: std::sync::Once = std::sync::Once::new();
