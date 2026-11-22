//! What the inventory modules' unit tests share.

use juggler_adapter::inventory::{
    Applied, Detail, Item, ItemKind, McpServer, McpTransport, Undo, Value,
};

/// Another tool's MCP server `name`, run as `npx <package>` with `env`.
pub(crate) fn others_server(name: &str, package: &str, env: Vec<(String, Value)>) -> Item {
    let server = McpServer {
        transport: McpTransport::Stdio {
            command: "npx".into(),
            args: vec![package.into()],
            env,
        },
        oauth: false,
    };
    Item::own("other-tool", ItemKind::McpServer, name, Detail::Mcp(server))
}

/// Another tool's `events` server (`npx events-mcp`), its `TOKEN` a
/// secret, then `more` variables.
pub(crate) fn events_server(more: Vec<(String, Value)>) -> Item {
    let mut env = vec![("TOKEN".to_owned(), Value::Secret)];
    env.extend(more);
    others_server("events", "events-mcp", env)
}

/// Carries out `applied`'s undo, which takes settings out of a file.
///
/// # Panics
///
/// For any other undo, or when taking them out fails.
pub(crate) fn undo_setting(applied: Applied) {
    let setting = match applied.undo {
        Undo::RemoveSetting {
            file,
            format,
            entries,
        } => Some((file, format, entries)),
        _ => None,
    };
    let (file, format, entries) = setting.expect("a setting's undo");
    crate::config_edit::remove(&file, format, &entries).expect("taken out");
}
