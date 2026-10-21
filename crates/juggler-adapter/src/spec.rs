//! ADR-0019 §1: tools are data. Each adapter describes its tool in a
//! [`ToolSpec`], and every other layer (the commands, the UI through
//! `tools_catalog`) reads the spec instead of branching on a tool id.
//! Domain data only: the app maps it to its wire DTO. What the tool can
//! do stays [`crate::Capabilities`], and how it is isolated stays the
//! adapter's isolation keys; the catalog composes the three.

/// How a tool is used: a command-line tool in a terminal, or a desktop
/// app in its own window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolKind {
    /// Runs in a terminal.
    Cli,
    /// Runs as its own desktop app.
    Desktop,
}

/// What first run calls one kind of setup, and how a found setup is
/// recognised as that kind. The first group that matches wins; a group
/// with neither test catches the rest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SetupGroup {
    /// Stable within the tool (`profiles`, `default`, `other`).
    pub id: String,
    /// The group's heading.
    pub title: String,
    /// Matches a setup whose path contains this.
    pub path_contains: Option<String>,
    /// Matches a setup the scan labelled with this origin
    /// (`vendor-default`).
    pub origin: Option<String>,
    /// The group's heading says which version of the tool is installed.
    pub shows_install: bool,
}

/// The words for a tool's own sign-in, so no screen spells them per tool.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SignInWords {
    /// First mention on a surface.
    pub own: String,
    /// In a list of tools.
    pub short: String,
}

/// Where the tool sits in the app's flows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ToolFlags {
    /// First run on a fresh install brings this tool's setups in.
    pub first_run: bool,
    /// The tool a fresh account or a folder import is for when nothing
    /// else says (one tool holds this).
    pub primary: bool,
}

/// Everything a screen or a command needs to know about one tool,
/// supplied by its adapter. Static and versioned with the adapter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolSpec {
    /// The tool's id (the adapter's [`crate::HarnessAdapter::id`]).
    pub id: String,
    /// Its name on every screen.
    pub display_name: String,
    /// Who makes it; empty when unknown.
    pub vendor: String,
    /// The tool's mark (an icon asset id); `None` draws the letter mark.
    pub mark: Option<String>,
    /// The vendor's mark, where it differs from the tool's.
    pub vendor_mark: Option<String>,
    /// Where to get it; `None` when there is no public page.
    pub install_url: Option<String>,
    /// A one-line install command, where the vendor documents one.
    pub install_command: Option<String>,
    /// The tool's own sign-in, in words.
    pub sign_in: SignInWords,
    /// What "the same sign-in, settings and history" is the same as.
    pub same_as: String,
    /// The desktop app that can hold the same setup open, by name, when
    /// there is one.
    pub desktop_app: Option<String>,
    /// How it is used.
    pub kinds: Vec<ToolKind>,
    /// The project rules files it reads.
    pub rules_files: Vec<String>,
    /// First run's words while it looks for this tool's setups.
    pub scan_hint: String,
    /// First run's groups for this tool's setups, in order.
    pub setup_groups: Vec<SetupGroup>,
    /// The name its default setup is brought in as; `None` uses the
    /// folder's name.
    pub default_label: Option<String>,
    /// Where it sits in the app's flows.
    pub flags: ToolFlags,
    /// Terms the user should know.
    pub terms_notes: Vec<String>,
}

impl ToolSpec {
    /// A tool no adapter described beyond its id: the id as its name, the
    /// letter mark, one group for every setup, and nothing claimed.
    #[must_use]
    pub fn generic(id: &str) -> Self {
        Self {
            id: id.to_owned(),
            display_name: id.to_owned(),
            vendor: String::new(),
            mark: None,
            vendor_mark: None,
            install_url: None,
            install_command: None,
            sign_in: SignInWords {
                own: format!("{id}'s own sign-in"),
                short: format!("{id}'s"),
            },
            same_as: "as wherever you use it now".to_owned(),
            desktop_app: None,
            kinds: vec![ToolKind::Cli],
            rules_files: Vec::new(),
            scan_hint:
                "Your default setup and anything else that looks like one. Nothing is changed."
                    .to_owned(),
            setup_groups: vec![SetupGroup {
                id: "setups".to_owned(),
                title: format!("{id} setups"),
                path_contains: None,
                origin: None,
                shows_install: true,
            }],
            default_label: None,
            flags: ToolFlags::default(),
            terms_notes: Vec::new(),
        }
    }
}
