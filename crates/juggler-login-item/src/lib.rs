//! Open AI Juggler at login (AJ-E1509 Req.4; decision D-26, 2026-10-22,
//! the maintainer's approval of `SMAppService` through
//! `objc2-service-management` 0.3.2).
//!
//! THE AUDIT: macOS's own way to open an app at login is `SMAppService`
//! (macOS 13 and later; AI Juggler needs 14). Every call to it is `unsafe`
//! in its Rust bindings, for the generated bindings' blanket reason, not
//! for any pointer or lifetime the caller manages: each takes only its
//! receiver, and the error out-parameter is turned into a `Result` by the
//! binding. This crate makes exactly those four calls, each with its
//! SAFETY note, and gives the app a safe API: [`status`] and [`set`].
//!
//! What this crate is NOT: a general unsafe escape hatch. It exists so the
//! workspace law `unsafe_code = "forbid"` stays absolute for every other
//! crate; it doesn't inherit the workspace lint baseline (see
//! `Cargo.toml`). Any other `unsafe` anywhere needs a new recorded
//! decision.
//!
//! Registering only works for the installed, signed app: a development
//! build (no bundle) reads [`LoginItem::Unavailable`].
#![deny(missing_docs, unsafe_op_in_unsafe_fn)]
#![cfg_attr(target_os = "macos", allow(unsafe_code))] // D-26: the four audited calls

/// Whether AI Juggler opens at login.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoginItem {
    /// It opens at login.
    On,
    /// It doesn't.
    Off,
    /// Registered, but waiting for the user's approval in System Settings
    /// › General › Login Items.
    NeedsApproval,
    /// Not here: a development build, or another system.
    Unavailable,
}

/// Whether AI Juggler opens at login now.
#[must_use]
pub fn status() -> LoginItem {
    imp::status()
}

/// Turns opening at login on or off; returns how it stands after.
///
/// # Errors
///
/// The system's own words when it refuses (an unsigned build, the user's
/// own setting).
pub fn set(on: bool) -> Result<LoginItem, String> {
    imp::set(on)
}

#[cfg(target_os = "macos")]
mod imp {
    use objc2_service_management::{SMAppService, SMAppServiceStatus};

    use super::LoginItem;

    fn service() -> objc2::rc::Retained<SMAppService> {
        // SAFETY (D-26 audit): `+[SMAppService mainAppService]` is a class
        // method with no arguments that returns the calling app's own
        // service object, retained; it has no preconditions on macOS 13+
        // (AI Juggler requires 14).
        unsafe { SMAppService::mainAppService() }
    }

    pub fn status() -> LoginItem {
        let service = service();
        // SAFETY (D-26 audit): `-status` only reads the service's
        // registration state; it takes no arguments and changes nothing.
        let raw = unsafe { service.status() };
        match raw {
            SMAppServiceStatus::Enabled => LoginItem::On,
            SMAppServiceStatus::NotRegistered => LoginItem::Off,
            SMAppServiceStatus::RequiresApproval => LoginItem::NeedsApproval,
            _ => LoginItem::Unavailable,
        }
    }

    pub fn set(on: bool) -> Result<LoginItem, String> {
        let service = service();
        let done = if on {
            // SAFETY (D-26 audit): `-registerAndReturnError:` takes only
            // the receiver; the binding owns the error out-parameter and
            // returns it as `Err`. It may show the system's notice; it has
            // no memory-safety preconditions.
            unsafe { service.registerAndReturnError() }
        } else {
            // SAFETY (D-26 audit): as `register`, for `-unregisterAndReturnError:`.
            unsafe { service.unregisterAndReturnError() }
        };
        done.map_err(|error| error.localizedDescription().to_string())?;
        Ok(status())
    }
}

#[cfg(not(target_os = "macos"))]
mod imp {
    use super::LoginItem;

    pub fn status() -> LoginItem {
        LoginItem::Unavailable
    }

    pub fn set(_on: bool) -> Result<LoginItem, String> {
        Err("Opening at login is only on macOS.".to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A development build has no bundle to register: the status says
    /// so, and asking never pretends it worked.
    #[test]
    fn an_unbundled_build_reads_unavailable_or_off_and_never_claims_on() {
        let now = status();
        assert_ne!(now, LoginItem::On, "a test binary never opens at login");
    }
}
