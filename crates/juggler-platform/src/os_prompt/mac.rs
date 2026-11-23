//! The macOS calls behind the gate: the only place in the workspace that
//! raises a system prompt directly (`scripts/os_prompt_lint.py`). Each is
//! reached through [`super`]'s gated functions, never on its own.

use objc2_foundation::{NSAppleEventDescriptor, NSAppleEventSendOptions, NSString};

use super::Status;

/// Whether this process may use accessibility features (never asks).
pub(super) fn accessibility_trusted() -> bool {
    macos_accessibility_client::accessibility::application_is_trusted()
}

/// Asks: the system's dialog adds AI Juggler to the Accessibility list.
pub(super) fn accessibility_ask() -> bool {
    macos_accessibility_client::accessibility::application_is_trusted_with_prompt()
}

/// Whether this process may record the screen (never asks; answered once
/// per process).
pub(super) fn screen_recording_allowed() -> bool {
    core_graphics::access::ScreenCaptureAccess.preflight()
}

/// Asks for Screen Recording.
pub(super) fn screen_recording_ask() -> bool {
    core_graphics::access::ScreenCaptureAccess.request()
}

/// `osascript`, by absolute path.
const OSASCRIPT: &str = "/usr/bin/osascript";

/// Runs `source` with `args` (`on run argv`), its output or its error.
pub(super) fn osascript(source: &str, args: &[&str]) -> Result<String, String> {
    let out = crate::process::helper(OSASCRIPT)
        .arg("-e")
        .arg(source)
        .arg("--")
        .args(args)
        .output()
        .map_err(|error| error.to_string())?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_owned())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_owned())
    }
}

/// `errAEEventWouldRequireUserConsent`: it would have asked.
const NEEDS_CONSENT: isize = -1744;
/// `errAEEventNotPermitted`: the user said no.
const NOT_PERMITTED: isize = -1743;
/// `errAEEventNotHandled`: delivered, and the app didn't know it.
const NOT_HANDLED: isize = -1708;
/// `kAEDoNotPromptForUserConsent` (macOS 10.14): never ask.
const NO_PROMPT: usize = 0x0002_0000;
/// A question no app answers: delivering it shows only whether it may be.
const PROBE: u32 = u32::from_be_bytes(*b"aijP");

/// Whether Apple events may go to the running app `bundle_id`, read with
/// an event that never asks (`None`: not running, or no answer).
pub(super) fn automation_status(bundle_id: &str) -> Option<Status> {
    let target =
        NSAppleEventDescriptor::descriptorWithBundleIdentifier(&NSString::from_str(bundle_id));
    let event = NSAppleEventDescriptor::appleEventWithEventClass_eventID_targetDescriptor_returnID_transactionID(
        PROBE,
        PROBE,
        Some(&target),
        -1,
        0,
    );
    let options = NSAppleEventSendOptions(
        NSAppleEventSendOptions::NoReply.0 | NSAppleEventSendOptions::NeverInteract.0 | NO_PROMPT,
    );
    match event.sendEventWithOptions_timeout_error(options, 2.0) {
        Ok(_) => Some(Status::Allowed),
        Err(error) => match error.code() {
            NOT_HANDLED => Some(Status::Allowed),
            NEEDS_CONSENT => Some(Status::NotAsked),
            NOT_PERMITTED => Some(Status::NotAllowed),
            _ => None,
        },
    }
}

/// The quarantine flags of a download at `path` (`None`: it carries none,
/// so the system won't ask when it opens).
pub(super) fn quarantine_flags(path: &std::path::Path) -> Option<u32> {
    let out = crate::process::helper("/usr/bin/xattr")
        .args(["-p", "com.apple.quarantine"])
        .arg(path)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    u32::from_str_radix(text.split(';').next()?.trim(), 16).ok()
}
