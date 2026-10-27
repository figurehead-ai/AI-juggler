//! ADR-0019 §3: feature areas as modules, each with its own commands,
//! DTOs and (where it keeps state) a service and a repository. New
//! features land here, not in `commands.rs`.

pub mod account_settings;
pub mod agent_bridge;
pub mod app;
pub mod app_icons;
pub mod budgets;
pub mod conversations;
pub mod fleet;
pub mod handover;
pub mod messages;
pub mod notifications;
pub mod outside_search;
pub mod packs;
pub mod pools;
pub mod processes;
pub mod project_sync;
pub mod projects;
pub mod quota;
pub mod runtime;
pub mod service_accounts;
pub mod sessions;
pub mod staging;
pub mod subagents;
pub mod subscriptions;
pub mod subtasks;
pub mod switch_setup;
pub mod tasks;
pub mod terminal;
pub mod tool_updates;
pub mod tools;
pub mod trust;
pub mod usage_history;
