//! The operating-system seams (the cross-platform audit's C5): the one
//! place where what differs between macOS, Linux and Windows lives, so
//! "where does the Windows version go?" has one answer.
//!
//! - [`user`]: who this process runs as.
//! - [`apps`]: starting an installed app (one of its instances).
//! - [`file_trust`]: whose a file is, and whether a program is safe to run.
//! - [`private_fs`]: folders and files only this user can open.
//! - [`local_ipc`]: a local connection only this user can make.
//! - [`system`]: what this system calls this computer and its user, and
//!   whether the app's icon can carry a count.
//! - [`room`]: free disk space and memory, for the local cache's size.
//!
//! Each seam has one implementation per platform. Where a platform has no
//! honest implementation yet, the seam says so (an `Unsupported` error, or
//! a type that can't be made there) rather than pretend: a check that
//! can't be made is a refusal, never a pass. On Windows today that is all
//! of [`user`], [`private_fs`] and [`local_ipc`]: telling who is on the
//! other end of a named pipe, or making a file owner-only, needs Win32
//! calls this workspace doesn't make without a vetted wrapper.
//!
//! This crate knows no tool, no product and no UI: adapters and features
//! call it, and keep their own words.

#![cfg_attr(
    test,
    allow(clippy::pedantic, clippy::unwrap_used, clippy::expect_used)
)]

pub mod apps;
pub mod file_trust;
pub mod local_ipc;
pub mod private_fs;
pub mod room;
pub mod system;
pub mod user;

/// The error a seam returns where this system has no implementation yet:
/// `what` isn't available here, said plainly.
#[must_use]
pub fn unsupported(what: &str) -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        format!("{what} isn't available on this system yet"),
    )
}

/// A type with no values: what a seam holds on a platform where it can't
/// be made, so code that would need one there can't be reached.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Never {}
