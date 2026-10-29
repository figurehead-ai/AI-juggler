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
    /// Its short name where room is tight (a rail row, a graph label).
    pub short_name: String,
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
    /// there is one (as the Dock names it: "Claude", `ChatGPT`).
    pub desktop_app: Option<String>,
    /// That app keeps a meter of the account's limits AI Juggler reads
    /// (the Claude app's), so a screen may say limits come from it.
    pub app_meter: bool,
    /// How it is used.
    pub kinds: Vec<ToolKind>,
    /// The project rules files it reads.
    pub rules_files: Vec<String>,
    /// How one of its rules files reads another file instead of repeating
    /// it, as a line with `{file}` for that file's name; `None` when the
    /// tool can't (AJ-E1507: a rules file that reads the master is in sync).
    pub rules_import: Option<String>,
    /// First run's words while it looks for this tool's setups.
    pub scan_hint: String,
    /// First run's groups for this tool's setups, in order.
    pub setup_groups: Vec<SetupGroup>,
    /// The name its default setup is brought in as; `None` uses the
    /// folder's name.
    pub default_label: Option<String>,
    /// Where it sits in the app's flows.
    pub flags: ToolFlags,
    /// Its vendor's terms, shown when an account is added (AJ-E1518).
    pub terms: Option<ToolTerms>,
    /// AJ-E1611: where its new versions are published, for the
    /// once-a-day check; `None` when there's no public list to ask.
    pub release_feed: Option<ReleaseFeed>,
    /// AJ-E1611: the command the user runs to update it themselves. AI
    /// Juggler shows it and never runs it.
    pub update_command: Option<String>,
}

/// AJ-E1611: a public list of a tool's released versions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReleaseFeed {
    /// An npm package: its `latest` version on npm's public registry.
    Npm {
        /// The package's name (`@scope/name` or `name`).
        package: String,
    },
}

/// A tool's own terms, and what they say about one person using several
/// accounts, shown when an account is added (AJ-E1518). What the vendor's
/// terms say is research: each line is recorded with its source and the
/// day it was checked in `docs/qa/evidence/AJ-E1518/terms-claims-register.md`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolTerms {
    /// The terms' name, as the vendor titles them.
    pub name: String,
    /// Where they are, on the vendor's own site.
    pub url: String,
    /// One line on what several accounts mean under them.
    pub accounts: String,
    /// The day the line was last checked against them (`YYYY-MM-DD`).
    pub checked: String,
}

impl ToolSpec {
    /// A tool no adapter described beyond its id: the id as its name, the
    /// letter mark, one group for every setup, and nothing claimed.
    #[must_use]
    pub fn generic(id: &str) -> Self {
        Self {
            id: id.to_owned(),
            display_name: id.to_owned(),
            short_name: id.to_owned(),
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
            app_meter: false,
            kinds: vec![ToolKind::Cli],
            rules_files: Vec::new(),
            rules_import: None,
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
            terms: None,
            release_feed: None,
            update_command: None,
        }
    }

    /// The line that makes one of this tool's rules files read `file`, if
    /// the tool has one.
    #[must_use]
    pub fn rules_import_line(&self, file: &str) -> Option<String> {
        self.rules_import
            .as_ref()
            .map(|template| template.replace("{file}", file))
    }

    /// Whether rules text of this tool reads `file` through its import
    /// line (a line of its own, spaces around it aside).
    #[must_use]
    pub fn rules_reads(&self, text: &str, file: &str) -> bool {
        self.rules_import_line(file)
            .is_some_and(|line| text.lines().any(|l| l.trim() == line))
    }
}

#[cfg(test)]
mod tests {
    use super::ToolSpec;

    #[test]
    fn a_rules_file_reads_another_only_through_a_line_of_its_own() {
        let mut spec = ToolSpec::generic("tool");
        assert_eq!(spec.rules_import_line("OTHER.md"), None);
        assert!(!spec.rules_reads("@OTHER.md", "OTHER.md"));
        spec.rules_import = Some("@{file}".to_owned());
        assert_eq!(
            spec.rules_import_line("OTHER.md").as_deref(),
            Some("@OTHER.md")
        );
        assert!(spec.rules_reads("# Rules\n\n  @OTHER.md  \n", "OTHER.md"));
        assert!(!spec.rules_reads("See @OTHER.md for more.", "OTHER.md"));
        assert!(!spec.rules_reads("@OTHER.md.bak", "OTHER.md"));
    }
}
