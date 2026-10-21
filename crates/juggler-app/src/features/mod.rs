//! ADR-0019 §3: feature areas as modules, each with its own commands,
//! DTOs and (where it keeps state) a service and a repository. New
//! features land here, not in `commands.rs`.

pub mod projects;
pub mod sessions;
pub mod tools;
