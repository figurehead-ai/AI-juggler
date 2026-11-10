//! AJ-E2019 (ADR-0029): the typed command map the UI calls through,
//! generated from the `app_commands!` registry.
//!
//! Like ts-rs's `export_bindings` tests, [`commands_ts_is_written`] writes
//! `ui/src/types/generated/commands.ts` when `cargo test` runs, and
//! `scripts/ts_types_fresh.sh` fails when what it wrote isn't committed.
//! Each registry entry names a command function; its signature is read
//! from its source with `syn`:
//!
//! - the arguments are its parameters minus the ones Tauri injects
//!   (`State`, `AppHandle`, the window), named as Tauri names them
//!   (camelCase, unless the command says `rename_all`);
//! - the result is `AppResult<T>`'s `T` (`null` for none);
//! - a struct or enum is the ts-rs type of that name in
//!   `ui/src/types/generated/`, so a type exists once.
//!
//! A type it can't map fails the generator, naming the command, instead
//! of becoming `unknown`; [`UNTYPED`] would list any that is untyped on
//! purpose, each with its reason.

use std::collections::{BTreeSet, HashMap};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use quote::ToTokens as _;
use syn::{FnArg, GenericArgument, Item, PathArguments, ReturnType, Type};

/// Types that are untyped JSON on purpose: `(Rust type, reason)`. None
/// today: every command's arguments and result have a type. A new entry
/// says why its JSON has no shape.
const UNTYPED: &[(&str, &str)] = &[];

/// Parameters Tauri fills itself: never sent by the page.
const INJECTED: &[&str] = &[
    "State",
    "AppHandle",
    "Window",
    "WebviewWindow",
    "Webview",
    "Request",
    "CommandScope",
    "GlobalScope",
];

/// One registered command, as the page calls it.
#[derive(Debug, PartialEq)]
pub(crate) struct Command {
    /// The name the page invokes (a `rename` honoured).
    pub wire: String,
    /// `module::function` in the registry.
    pub path: String,
    /// `Read` or `Write`.
    pub access: String,
    /// `(argument name, TypeScript type, optional)`.
    pub args: Vec<(String, String, bool)>,
    /// The result's TypeScript type.
    pub result: String,
}

/// A command function found in the source.
struct Found {
    sig: syn::Signature,
    rename: Option<String>,
    snake_args: bool,
}

/// What the type mapping needs and collects.
pub(crate) struct Types<'a> {
    /// The ts-rs types in `ui/src/types/generated/`, by name.
    pub exported: &'a BTreeSet<String>,
    /// The ones used, to import.
    pub used: BTreeSet<String>,
    /// A Tauri `Channel` is used, to import.
    pub channel: bool,
}

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `(access, module::function)` for every registry entry
/// ([`crate::COMMANDS`], the `app_commands!` registry), in order.
pub(crate) fn registry() -> Vec<(String, String)> {
    crate::COMMANDS
        .iter()
        .map(|(path, access)| (format!("{access:?}"), path.split_whitespace().collect()))
        .collect()
}

/// Every `.rs` file under `dir`, with its module path (`a::b`).
fn sources(dir: &Path) -> Vec<(String, PathBuf)> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(at) = stack.pop() {
        for entry in std::fs::read_dir(&at).expect("src/").flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "rs") {
                let relative = path.strip_prefix(dir).expect("under src/");
                let mut parts: Vec<String> = relative
                    .with_extension("")
                    .iter()
                    .map(|p| p.to_string_lossy().into_owned())
                    .collect();
                if parts.last().is_some_and(|l| l == "mod" || l == "lib") {
                    parts.pop();
                }
                if parts == ["main"] {
                    continue;
                }
                out.push((parts.join("::"), path));
            }
        }
    }
    out.sort();
    out
}

fn is_cfg_test(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|a| {
        a.path().is_ident("cfg") && {
            let text = a.meta.to_token_stream().to_string();
            text.contains("test") && !text.contains("not (test")
        }
    })
}

/// The `#[tauri::command]` attribute's options, when `attrs` has one.
fn command_attr(attrs: &[syn::Attribute]) -> Option<(Option<String>, bool)> {
    let attr = attrs.iter().find(|a| {
        let segments: Vec<String> = a
            .path()
            .segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect();
        segments == ["tauri", "command"] || segments == ["command"]
    })?;
    let mut rename = None;
    let mut snake = false;
    if matches!(attr.meta, syn::Meta::List(_)) {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("rename") {
                rename = Some(meta.value()?.parse::<syn::LitStr>()?.value());
            } else if meta.path.is_ident("rename_all") {
                snake = meta.value()?.parse::<syn::LitStr>()?.value() == "snake_case";
            } else if meta.input.peek(syn::Token![=]) {
                meta.value()?.parse::<syn::Expr>()?;
            }
            Ok(())
        })
        .expect("a #[tauri::command(…)] the generator reads");
    }
    Some((rename, snake))
}

fn collect(module: &str, items: &[Item], into: &mut HashMap<String, Vec<(String, Found)>>) {
    for item in items {
        match item {
            Item::Fn(function) if !is_cfg_test(&function.attrs) => {
                if let Some((rename, snake_args)) = command_attr(&function.attrs) {
                    let name = function.sig.ident.to_string();
                    let path = if module.is_empty() {
                        name.clone()
                    } else {
                        format!("{module}::{name}")
                    };
                    into.entry(name).or_default().push((
                        path,
                        Found {
                            sig: function.sig.clone(),
                            rename,
                            snake_args,
                        },
                    ));
                }
            }
            Item::Mod(inner) if !is_cfg_test(&inner.attrs) => {
                if let Some((_, items)) = &inner.content {
                    let path = if module.is_empty() {
                        inner.ident.to_string()
                    } else {
                        format!("{module}::{}", inner.ident)
                    };
                    collect(&path, items, into);
                }
            }
            _ => {}
        }
    }
}

/// Tauri's argument name: `conversation_id` → `conversationId`.
pub(crate) fn camel(name: &str) -> String {
    let mut out = String::new();
    for (i, part) in name.split('_').filter(|p| !p.is_empty()).enumerate() {
        if i == 0 {
            out.push_str(part);
        } else {
            let mut chars = part.chars();
            if let Some(first) = chars.next() {
                out.extend(first.to_uppercase());
                out.push_str(chars.as_str());
            }
        }
    }
    out
}

fn type_args(segment: &syn::PathSegment) -> Vec<&Type> {
    match &segment.arguments {
        PathArguments::AngleBracketed(args) => args
            .args
            .iter()
            .filter_map(|a| match a {
                GenericArgument::Type(t) => Some(t),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn text(ty: &Type) -> String {
    ty.to_token_stream()
        .to_string()
        .replace(" :: ", "::")
        .replace(" < ", "<")
        .replace(" >", ">")
        .replace("& ", "&")
        .replace(" ,", ",")
}

fn strip_ref(ty: &Type) -> &Type {
    match ty {
        Type::Reference(r) => strip_ref(&r.elem),
        Type::Paren(p) => strip_ref(&p.elem),
        Type::Group(g) => strip_ref(&g.elem),
        other => other,
    }
}

fn last_segment(ty: &Type) -> Option<&syn::PathSegment> {
    match strip_ref(ty) {
        Type::Path(p) if p.qself.is_none() => p.path.segments.last(),
        _ => None,
    }
}

/// `T[]`, parenthesised when `T` is a union.
fn array(inner: &str) -> String {
    if inner.contains(' ') && !inner.starts_with('[') && !inner.starts_with('{') {
        format!("({inner})[]")
    } else {
        format!("{inner}[]")
    }
}

impl<'a> Types<'a> {
    pub(crate) fn new(exported: &'a BTreeSet<String>) -> Self {
        Self {
            exported,
            used: BTreeSet::new(),
            channel: false,
        }
    }

    /// The TypeScript for `ty`, or why it has none.
    pub(crate) fn ts(&mut self, ty: &Type) -> Result<String, String> {
        let ty = strip_ref(ty);
        match ty {
            Type::Tuple(tuple) if tuple.elems.is_empty() => Ok("null".into()),
            Type::Tuple(tuple) => {
                let parts: Result<Vec<String>, String> =
                    tuple.elems.iter().map(|t| self.ts(t)).collect();
                Ok(format!("[{}]", parts?.join(", ")))
            }
            Type::Slice(slice) => Ok(array(&self.ts(&slice.elem)?)),
            Type::Array(arr) => Ok(array(&self.ts(&arr.elem)?)),
            Type::Path(path) if path.qself.is_none() => self.path(ty, &path.path),
            _ => Err(format!("`{}` has no TypeScript form", text(ty))),
        }
    }

    fn path(&mut self, ty: &Type, path: &syn::Path) -> Result<String, String> {
        let full = text(ty);
        if UNTYPED.iter().any(|(name, _)| *name == full) {
            return Ok("unknown".into());
        }
        let segment = path.segments.last().ok_or("an empty path")?;
        let args = type_args(segment);
        let one = |types: &mut Self| -> Result<String, String> {
            match args.as_slice() {
                [only] => types.ts(only),
                _ => Err(format!("`{full}` takes one type")),
            }
        };
        let name = segment.ident.to_string();
        Ok(match name.as_str() {
            "String" | "str" | "PathBuf" | "Path" | "char" => "string".into(),
            "bool" => "boolean".into(),
            "u8" | "u16" | "u32" | "u64" | "u128" | "usize" | "i8" | "i16" | "i32" | "i64"
            | "i128" | "isize" | "f32" | "f64" => "number".into(),
            "Option" => {
                let inner = one(self)?;
                if inner.ends_with(" | null") {
                    inner
                } else {
                    format!("{inner} | null")
                }
            }
            "Vec" | "VecDeque" | "HashSet" | "BTreeSet" => array(&one(self)?),
            // `Redacted` is `#[serde(transparent)]`: a secret sent as its text.
            "Box" | "Arc" | "Rc" | "Cow" | "Redacted" => one(self)?,
            "HashMap" | "BTreeMap" => match args.as_slice() {
                [key, value] if self.ts(key)? == "string" => {
                    format!("Record<string, {}>", self.ts(value)?)
                }
                _ => return Err(format!("`{full}`: only string-keyed maps are mapped")),
            },
            _ if self.exported.contains(&name) => {
                self.used.insert(name.clone());
                if args.is_empty() {
                    name
                } else {
                    let parts: Result<Vec<String>, String> =
                        args.iter().map(|t| self.ts(t)).collect();
                    format!("{name}<{}>", parts?.join(", "))
                }
            }
            _ => {
                return Err(format!(
                    "`{full}` isn't exported by ts-rs to ui/src/types/generated/ (derive TS with \
                     #[ts(export)], or list it in UNTYPED with a reason)"
                ));
            }
        })
    }

    /// The result: `AppResult<T>` / `Result<T, _>` is `T`. A command that
    /// answers nothing (`()`, sent as `null`) is `void`, as a TypeScript
    /// function that returns nothing is.
    fn result(&mut self, output: &ReturnType) -> Result<String, String> {
        let ReturnType::Type(_, ty) = output else {
            return Ok("void".into());
        };
        let value = match last_segment(ty) {
            Some(seg) if seg.ident == "AppResult" || seg.ident == "Result" => {
                let args = type_args(seg);
                let ok = args
                    .first()
                    .ok_or_else(|| format!("`{}` has no value type", text(ty)))?;
                self.ts(ok)?
            }
            _ => self.ts(ty)?,
        };
        Ok(if value == "null" {
            "void".into()
        } else {
            value
        })
    }
}

fn injected(ty: &Type) -> bool {
    last_segment(ty).is_some_and(|s| INJECTED.iter().any(|i| s.ident == i))
}

/// A Tauri `Channel<T>`: the page passes one (see `lib/ipc/transport.ts`'s
/// `channel`), so it is an argument, typed as `@tauri-apps/api`'s.
fn tauri_channel(ty: &Type) -> Option<&Type> {
    let Type::Path(path) = strip_ref(ty) else {
        return None;
    };
    let segment = path.path.segments.last()?;
    let from_tauri = path.path.segments.len() == 1
        || path
            .path
            .segments
            .iter()
            .any(|s| s.ident == "ipc" || s.ident == "tauri");
    (segment.ident == "Channel" && from_tauri)
        .then(|| type_args(segment).first().copied())
        .flatten()
}

/// One command's entry, or why it has none.
fn command(
    access: &str,
    path: &str,
    found: &Found,
    types: &mut Types<'_>,
) -> Result<Command, String> {
    let name = path.rsplit("::").next().unwrap_or(path);
    let wire = found.rename.clone().unwrap_or_else(|| name.to_owned());
    let mut args = Vec::new();
    for input in &found.sig.inputs {
        let FnArg::Typed(typed) = input else {
            return Err(format!("{wire}: takes `self`"));
        };
        if injected(&typed.ty) {
            continue;
        }
        let syn::Pat::Ident(ident) = &*typed.pat else {
            return Err(format!("{wire}: a parameter isn't a plain name"));
        };
        let rust = ident.ident.to_string();
        let arg = if found.snake_args {
            rust.trim_start_matches('_').to_owned()
        } else {
            camel(&rust)
        };
        let (ts, optional) = if let Some(message) = tauri_channel(&typed.ty) {
            types.channel = true;
            (
                format!(
                    "Channel<{}>",
                    types
                        .ts(message)
                        .map_err(|e| format!("{wire}({rust}): {e}"))?
                ),
                false,
            )
        } else {
            let optional = last_segment(&typed.ty).is_some_and(|s| s.ident == "Option");
            (
                types
                    .ts(&typed.ty)
                    .map_err(|e| format!("{wire}({rust}): {e}"))?,
                optional,
            )
        };
        args.push((arg, ts, optional));
    }
    let result = types
        .result(&found.sig.output)
        .map_err(|e| format!("{wire} (result): {e}"))?;
    Ok(Command {
        wire,
        path: path.to_owned(),
        access: access.to_owned(),
        args,
        result,
    })
}

/// Every registered command, typed; the problems, when any.
pub(crate) fn commands(exported: &BTreeSet<String>) -> (Vec<Command>, Types<'_>, Vec<String>) {
    let found = found_commands();
    let mut types = Types::new(exported);
    let mut out = Vec::new();
    let mut problems = Vec::new();
    for (access, path) in registry() {
        let name = path.rsplit("::").next().unwrap_or(&path).to_owned();
        let candidates = found.get(&name).map(Vec::as_slice).unwrap_or_default();
        // By its module path; else the one command of that name (a
        // `pub use` re-export).
        let at = candidates
            .iter()
            .find(|(p, _)| *p == path)
            .or(match candidates {
                [only] => Some(only),
                _ => None,
            });
        match at {
            Some((_, f)) => match command(&access, &path, f, &mut types) {
                Ok(c) => out.push(c),
                Err(e) => problems.push(e),
            },
            None => problems.push(format!(
                "{path}: no single #[tauri::command] fn {name} under src/ ({} found)",
                candidates.len()
            )),
        }
    }
    out.sort_by(|a, b| a.wire.cmp(&b.wire));
    (out, types, problems)
}

/// The TypeScript property name: quoted when it isn't an identifier.
fn key(name: &str) -> String {
    if name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        name.to_owned()
    } else {
        format!("\"{name}\"")
    }
}

/// `commands.ts`'s text.
pub(crate) fn render(commands: &[Command], types: &Types<'_>) -> String {
    let mut out = String::from(
        "// Generated by crates/juggler-app/src/command_types.rs from the app_commands! registry\n\
         // (AJ-E2019, ADR-0029). Do not edit: `cargo test -p juggler-app --lib command_types` writes it.\n\n",
    );
    if types.channel {
        out.push_str("import type { Channel } from \"@tauri-apps/api/core\";\n");
    }
    for name in &types.used {
        let _ = writeln!(out, "import type {{ {name} }} from \"./{name}\";");
    }
    out.push_str(
        "\n/** Every command the backend serves: its arguments (as Tauri names them) and its result. */\n\
         export interface Commands {\n",
    );
    for c in commands {
        let _ = writeln!(out, "  /** {} {} */", c.access, c.path);
        let args = if c.args.is_empty() {
            "Record<string, never>".to_owned()
        } else {
            let fields: Vec<String> = c
                .args
                .iter()
                .map(|(n, t, optional)| {
                    format!("{}{}: {t}", key(n), if *optional { "?" } else { "" })
                })
                .collect();
            format!("{{ {} }}", fields.join("; "))
        };
        let _ = writeln!(
            out,
            "  {}: {{ args: {args}; result: {} }};",
            key(&c.wire),
            c.result
        );
    }
    out.push_str("}\n");
    out
}

/// The ts-rs types already written, by name.
pub(crate) fn exported(generated: &Path) -> BTreeSet<String> {
    std::fs::read_dir(generated)
        .expect("ui/src/types/generated/")
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            name.strip_suffix(".ts")
                .filter(|n| *n != "commands")
                .map(str::to_owned)
        })
        .collect()
}

/// The domain types (juggler-core, `ts` feature) a command serves as
/// they are: their TypeScript is written here, as their crate has no
/// export tests of its own in the app's build.
fn export_domain_types() {
    use juggler_core::{backup_archive, cost_ledger, recommendation, storage};
    use ts_rs::TS as _;
    let cfg = ts_rs::Config::from_env();
    backup_archive::BackupManifestInfo::export_all(&cfg).expect("BackupManifestInfo.ts");
    cost_ledger::LedgerPriceRow::export_all(&cfg).expect("LedgerPriceRow.ts");
    cost_ledger::PricingCatalog::export_all(&cfg).expect("PricingCatalog.ts");
    recommendation::Recommendation::export_all(&cfg).expect("Recommendation.ts");
    storage::CleanupTierSettings::export_all(&cfg).expect("CleanupTierSettings.ts");
}

/// Every struct or enum the app's source exports with ts-rs.
fn exported_in_source() -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for (_, file) in sources(&crate_dir().join("src")) {
        let text = std::fs::read_to_string(file).expect("a source file");
        let mut exporting = false;
        for line in text.lines().map(str::trim) {
            if line.starts_with("#[ts(") && line.contains("export") {
                exporting = true;
            } else if exporting && (line.starts_with("pub ") || line.starts_with('}')) {
                if let Some(rest) = line
                    .strip_prefix("pub struct ")
                    .or_else(|| line.strip_prefix("pub enum "))
                {
                    let name: String = rest
                        .chars()
                        .take_while(|c| c.is_alphanumeric() || *c == '_')
                        .collect();
                    out.insert(name);
                }
                exporting = false;
            }
        }
    }
    out
}

fn generated_dir() -> PathBuf {
    crate_dir().join("../../ui/src/types/generated")
}

/// Every `#[tauri::command]` fn under `src/`, by function name.
#[allow(
    clippy::panic,
    reason = "test-only: a source it can't parse stops the test, by file"
)]
fn found_commands() -> HashMap<String, Vec<(String, Found)>> {
    let mut found = HashMap::new();
    for (module, file) in sources(&crate_dir().join("src")) {
        let source = std::fs::read_to_string(&file).expect("a source file");
        let parsed = syn::parse_file(&source).unwrap_or_else(|e| panic!("{}: {e}", file.display()));
        collect(&module, &parsed.items, &mut found);
    }
    found
}

/// `ident::path::Type` → `Type`: the dev bridge names types by whatever
/// path it has in scope.
fn short(text: &str) -> String {
    let mut out = String::new();
    let mut word = String::new();
    let mut chars = text.chars().filter(|c| !c.is_whitespace()).peekable();
    while let Some(c) = chars.next() {
        if c.is_alphanumeric() || c == '_' {
            word.push(c);
        } else if c == ':' && chars.peek() == Some(&':') {
            chars.next();
            word.clear();
        } else {
            out.push_str(&word);
            word.clear();
            out.push(c);
        }
    }
    out + &word
}

/// One dev bridge route: `(ipc name, module::function, [(argument, type)])`.
type Route = (String, String, Vec<(String, String)>);

/// The dev bridge's routes (`dev_bridge/routes.rs`).
fn bridge_routes(routes_rs: &str) -> Vec<Route> {
    let start = routes_rs.find("read_routes! {").expect("the route table");
    let body = &routes_rs[start + "read_routes! {".len()..];
    let body = &body[..body.find("\n}\n").expect("the table's end")];
    let joined: String = body
        .lines()
        .map(str::trim)
        .filter(|l| !l.starts_with("//"))
        .collect::<Vec<_>>()
        .join(" ");
    // Entries end with `;` outside the call's parentheses.
    let mut entries = Vec::new();
    let (mut depth, mut current) = (0, String::new());
    for c in joined.chars() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            ';' if depth == 0 => {
                entries.push(std::mem::take(&mut current));
                continue;
            }
            _ => {}
        }
        current.push(c);
    }
    let mut out = Vec::new();
    for entry in entries
        .iter()
        .map(|e| e.trim())
        .filter(|e| e.starts_with('"'))
    {
        let ipc = entry.split('"').nth(1).expect("an ipc name").to_owned();
        let call = entry.split("=>").nth(1).expect("=>").trim();
        let call = call.split_once(' ').expect("a kind").1;
        let (path, rest) = call.split_once('(').expect("a call");
        let mut depth = 1;
        let end = rest
            .char_indices()
            .find(|&(_, c)| {
                match c {
                    '(' => depth += 1,
                    ')' => depth -= 1,
                    _ => {}
                }
                depth == 0
            })
            .expect("the call's end")
            .0;
        let inside = &rest[..end];
        let mut args = Vec::new();
        if let Some((_, list)) = inside.split_once(';') {
            let mut depth = 0;
            let mut current = String::new();
            for c in list.chars().chain(std::iter::once(',')) {
                match c {
                    '<' | '(' => depth += 1,
                    '>' | ')' => depth -= 1,
                    _ => {}
                }
                if c == ',' && depth == 0 {
                    let (name, ty) = current.split_once(':').expect("name: Type");
                    args.push((name.trim().to_owned(), short(ty)));
                    current.clear();
                } else {
                    current.push(c);
                }
            }
        }
        out.push((ipc, path.trim().to_owned(), args));
    }
    out
}

/// Where each route disagrees with the command it calls.
fn bridge_mismatches() -> Vec<String> {
    let found = found_commands();
    let routes =
        std::fs::read_to_string(crate_dir().join("src/dev_bridge/routes.rs")).expect("routes.rs");
    let mut problems = Vec::new();
    for (ipc, path, args) in bridge_routes(&routes) {
        let name = path.rsplit("::").next().unwrap_or(&path);
        let candidates = found.get(name).map(Vec::as_slice).unwrap_or_default();
        let Some((_, f)) = candidates
            .iter()
            .find(|(p, _)| *p == path)
            .or(match candidates {
                [only] => Some(only),
                _ => None,
            })
        else {
            problems.push(format!("{ipc}: no command {path}"));
            continue;
        };
        let wire = f.rename.clone().unwrap_or_else(|| name.to_owned());
        if wire != ipc {
            problems.push(format!("{ipc}: the command is invoked as {wire}"));
        }
        let params: Vec<(String, String)> = f
            .sig
            .inputs
            .iter()
            .filter_map(|input| match input {
                FnArg::Typed(t) if !injected(&t.ty) => match &*t.pat {
                    syn::Pat::Ident(i) => Some((i.ident.to_string(), short(&text(&t.ty)))),
                    _ => None,
                },
                _ => None,
            })
            .collect();
        if params != args {
            problems.push(format!(
                "{ipc}: the route passes {args:?}, the command takes {params:?}"
            ));
        }
    }
    problems
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Writes `commands.ts` (like ts-rs's `export_bindings` tests);
    /// `scripts/ts_types_fresh.sh` fails when it isn't committed.
    #[test]
    fn commands_ts_is_written() {
        export_domain_types();
        let generated = generated_dir();
        let mut exported = exported(&generated);
        // A type exported by a test running beside this one may not be
        // written yet: what the source exports counts too.
        exported.extend(exported_in_source());
        let (commands, types, problems) = commands(&exported);
        assert!(
            problems.is_empty(),
            "commands the generator can't type:\n{}",
            problems.join("\n")
        );
        assert_eq!(
            commands.len(),
            registry().len(),
            "every registered command is typed"
        );
        let text = render(&commands, &types);
        let path = generated.join("commands.ts");
        if std::fs::read_to_string(&path).ok().as_deref() != Some(text.as_str()) {
            std::fs::write(&path, text).expect("write commands.ts");
        }
    }

    /// A ts-rs export set of these names.
    fn names(list: &[&str]) -> BTreeSet<String> {
        list.iter().map(|s| (*s).to_owned()).collect()
    }

    fn mapped(rust: &str) -> Result<String, String> {
        let exported = names(&["KnownPlaceInfo", "Page"]);
        let mut types = Types::new(&exported);
        types.ts(&syn::parse_str::<Type>(rust).unwrap())
    }

    /// The mapping, one case per rule.
    #[test]
    fn types_map_as_the_adr_says() {
        for (rust, ts) in [
            ("String", "string"),
            ("&str", "string"),
            ("std::path::PathBuf", "string"),
            ("bool", "boolean"),
            ("u64", "number"),
            ("f32", "number"),
            ("Option<String>", "string | null"),
            ("Option<Option<u8>>", "number | null"),
            ("Vec<String>", "string[]"),
            ("Vec<Option<u32>>", "(number | null)[]"),
            ("(String, u32)", "[string, number]"),
            ("()", "null"),
            ("HashMap<String, Vec<bool>>", "Record<string, boolean[]>"),
            (
                "std::collections::BTreeMap<String, u8>",
                "Record<string, number>",
            ),
            ("Option<KnownPlaceInfo>", "KnownPlaceInfo | null"),
            (
                "crate::paging::Page<KnownPlaceInfo>",
                "Page<KnownPlaceInfo>",
            ),
            ("Arc<str>", "string"),
            ("juggler_core::redacted::Redacted<String>", "string"),
        ] {
            assert_eq!(mapped(rust).as_deref(), Ok(ts), "{rust}");
        }
        for rust in [
            "serde_json::Value",
            "NotExported",
            "HashMap<u32, String>",
            "impl Fn()",
            "Value",
            "Vec<NotExported>",
        ] {
            assert!(mapped(rust).is_err(), "{rust} maps to nothing");
        }
    }

    /// A command's entry: injected parameters left out, arguments
    /// camelCase (or as `rename_all` says), `Option` ones optional,
    /// `AppResult<T>` read as `T`, a rename honoured.
    #[test]
    fn a_command_reads_as_tauri_calls_it() {
        let exported = names(&["KnownPlaceInfo"]);
        let mut types = Types::new(&exported);
        let file: syn::File = syn::parse_str(
            "#[tauri::command(rename = \"places_seek\")]
             pub async fn places_find(state: tauri::State<'_, Arc<AppState>>, app: AppHandle,
                 conversation_id: String, account: Option<String>,
                 channel: tauri::ipc::Channel<KnownPlaceInfo>) -> AppResult<Option<KnownPlaceInfo>> { todo!() }
             #[tauri::command(rename_all = \"snake_case\")]
             fn quiet(window: tauri::Window, some_flag: bool) {}
             #[cfg(test)] mod tests { #[tauri::command] fn ignored() {} }",
        )
        .unwrap();
        let mut found = HashMap::new();
        collect("features::places", &file.items, &mut found);
        assert!(
            !found.contains_key("ignored"),
            "a test module's commands aren't read"
        );
        let (path, f) = &found["places_find"][0];
        assert_eq!(path, "features::places::places_find");
        let c = command("Read", path, f, &mut types).unwrap();
        assert_eq!(c.wire, "places_seek");
        assert_eq!(
            c.args,
            vec![
                ("conversationId".to_owned(), "string".to_owned(), false),
                ("account".to_owned(), "string | null".to_owned(), true),
                (
                    "channel".to_owned(),
                    "Channel<KnownPlaceInfo>".to_owned(),
                    false
                ),
            ]
        );
        assert_eq!(c.result, "KnownPlaceInfo | null");
        let (path, f) = &found["quiet"][0];
        let quiet = command("Write", path, f, &mut types).unwrap();
        assert_eq!(
            quiet.args,
            vec![("some_flag".to_owned(), "boolean".to_owned(), false)]
        );
        assert_eq!(quiet.result, "void");
        let text = render(&[c, quiet], &types);
        assert!(text.contains("import type { Channel } from \"@tauri-apps/api/core\";"));
        assert!(text.contains("import type { KnownPlaceInfo } from \"./KnownPlaceInfo\";"));
        assert!(text.contains(
            "  places_seek: { args: { conversationId: string; account?: string | null; channel: Channel<KnownPlaceInfo> }; result: KnownPlaceInfo | null };"
        ));
        assert!(text.contains("  quiet: { args: { some_flag: boolean }; result: void };"));
    }

    /// The dev bridge's routes call each command with its own argument
    /// list (0.4.1: `places_find` gained two, and only a lab-bridge build
    /// noticed).
    #[test]
    fn bridge_routes_match_the_commands() {
        let problems = bridge_mismatches();
        assert!(
            problems.is_empty(),
            "dev bridge routes out of step:\n{}",
            problems.join("\n")
        );
    }

    #[test]
    fn a_route_is_read_as_written() {
        let routes = bridge_routes(
            "read_routes! {\n    \"a\" => async_result x::y::a(state; id: String, page: crate::paging::PageRequest);\n    // note\n    \"b\" => value z::b() ;\n    \"c\" => result c(state; m: HashMap<String, u8>)\n        if m.is_empty();\n}\n",
        );
        assert_eq!(
            routes,
            vec![
                (
                    "a".into(),
                    "x::y::a".into(),
                    vec![
                        ("id".into(), "String".into()),
                        ("page".into(), "PageRequest".into())
                    ]
                ),
                ("b".into(), "z::b".into(), vec![]),
                (
                    "c".into(),
                    "c".into(),
                    vec![("m".into(), "HashMap<String,u8>".into())]
                ),
            ]
        );
        assert_eq!(
            short("Option < crate :: paging :: PageRequest >"),
            "Option<PageRequest>"
        );
    }

    /// A command whose type has no TypeScript form fails by name.
    #[test]
    fn an_unmapped_type_names_its_command() {
        let exported = BTreeSet::new();
        let mut types = Types::new(&exported);
        let file: syn::File =
            syn::parse_str("#[tauri::command] fn opaque(thing: Secret) {}").unwrap();
        let mut found = HashMap::new();
        collect("x", &file.items, &mut found);
        let (path, f) = &found["opaque"][0];
        let error = command("Read", path, f, &mut types).unwrap_err();
        assert!(error.starts_with("opaque(thing): `Secret`"), "{error}");
    }
}
