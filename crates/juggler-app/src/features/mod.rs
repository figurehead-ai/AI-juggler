//! ADR-0019 §3: feature areas as modules, each with its own commands,
//! DTOs and (where it keeps state) a service and a repository. New
//! features land here, not in `commands.rs`.

pub mod app;
pub mod conversations;
pub mod project_sync;
pub mod projects;
pub mod quota;
pub mod sessions;
pub mod switch_setup;
pub mod terminal;
pub mod tools;
