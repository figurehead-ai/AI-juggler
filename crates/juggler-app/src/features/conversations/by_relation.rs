//! AJ-E3102: the conversations other apps started, indexed by project,
//! App, tool and folder, so a page's list or count of them narrows by a
//! lookup instead of walking every conversation. Made from the rows the
//! lists show ([`super::commands::list_indexed`], only those AI Juggler
//! didn't start), with the rail's rule for a row's App
//! ([`crate::features::places::app_of`]), so a count and its list never
//! disagree. The relation-count cache keeps it, rebuilt when the index,
//! the places or the projects change (`features::relations`).

use std::collections::HashMap;

use super::dto::ConversationInfo;

/// One conversation's relations and state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Facet {
    /// The folder (config folder) whose files hold it.
    pub folder: String,
    /// The tool's id for it.
    pub id: String,
    /// The tool.
    pub tool: String,
    /// Its project, when its folder is in one.
    pub project: Option<String>,
    /// The App it's in: the desktop App it was last seen open in, else
    /// its folder's CLI App. 0.4.3: none when that App is a desktop app
    /// whose own records don't list it, or (D-102) it's a script's run
    /// ([`ConversationInfo::app_unlisted`]): no App's list or count has it,
    /// so a profile's equal the app's own records.
    pub app: Option<String>,
    /// Open now.
    pub live: bool,
    /// Archived in AI Juggler (0.4.1), and not open.
    pub archived: bool,
    /// When it was last active (else started), epoch ms.
    pub last_ms: i64,
}

/// The index: every outside conversation's [`Facet`], and where each is
/// by project, App, tool and folder.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ByRelation {
    facets: Vec<Facet>,
    by_project: HashMap<String, Vec<usize>>,
    by_app: HashMap<String, Vec<usize>>,
    by_tool: HashMap<String, Vec<usize>>,
    by_folder: HashMap<String, Vec<usize>>,
}

impl ByRelation {
    /// The index of `conversations` (those with an AI Juggler session are
    /// left out: they're counted as that session), each in the App
    /// `desktop` (`<tool>/<id>` → App) last saw it in.
    #[must_use]
    pub fn of(
        conversations: impl IntoIterator<Item = ConversationInfo>,
        desktop: &crate::features::places::DesktopApps,
    ) -> Self {
        let mut index = Self::default();
        for conversation in conversations {
            if conversation.session_id.is_some() {
                continue;
            }
            let app = (!conversation.app_unlisted).then(|| conversation.owner(desktop));
            let at = index.facets.len();
            if let Some(project) = &conversation.project_id {
                index
                    .by_project
                    .entry(project.clone())
                    .or_default()
                    .push(at);
            }
            if let Some(app) = &app {
                index.by_app.entry(app.clone()).or_default().push(at);
            }
            index
                .by_tool
                .entry(conversation.tool.clone())
                .or_default()
                .push(at);
            index
                .by_folder
                .entry(conversation.account_id.clone())
                .or_default()
                .push(at);
            index.facets.push(Facet {
                last_ms: conversation
                    .last_activity_ms
                    .or(conversation.started_ms)
                    .unwrap_or_default(),
                live: conversation.live,
                archived: !conversation.live && conversation.archived_here,
                project: conversation.project_id,
                app,
                tool: conversation.tool,
                folder: conversation.account_id,
                id: conversation.id,
            });
        }
        index
    }

    /// Every conversation in it.
    #[must_use]
    pub fn facets(&self) -> &[Facet] {
        &self.facets
    }

    /// Project `id`'s conversations.
    pub fn of_project<'a>(&'a self, id: &str) -> impl Iterator<Item = &'a Facet> {
        self.at(self.by_project.get(id))
    }

    /// App `id`'s conversations.
    pub fn of_app<'a>(&'a self, id: &str) -> impl Iterator<Item = &'a Facet> {
        self.at(self.by_app.get(id))
    }

    /// Tool `id`'s conversations.
    pub fn of_tool<'a>(&'a self, id: &str) -> impl Iterator<Item = &'a Facet> {
        self.at(self.by_tool.get(id))
    }

    /// Folder `id`'s conversations.
    pub fn of_folder<'a>(&'a self, id: &str) -> impl Iterator<Item = &'a Facet> {
        self.at(self.by_folder.get(id))
    }

    fn at<'a>(&'a self, rows: Option<&'a Vec<usize>>) -> impl Iterator<Item = &'a Facet> {
        rows.into_iter()
            .flatten()
            .filter_map(|at| self.facets.get(*at))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::places::DesktopApps;

    fn conversation(id: &str, folder: &str, project: Option<&str>) -> ConversationInfo {
        ConversationInfo {
            started_from: None,
            app: None,
            id: id.into(),
            tool: "claude-code".into(),
            account_id: folder.into(),
            account_label: folder.into(),
            project_id: project.map(Into::into),
            cwd: None,
            title: None,
            git_branch: None,
            started_in: None,
            started_ms: Some(1),
            last_activity_ms: Some(5),
            tokens: None,
            live: false,
            session_id: None,
            places: Vec::new(),
            pid: None,
            state: None,
            probably: false,
            viewed_ms: None,
            background: false,
            inactive: false,
            subtask_count: None,
            app_archived: false,
            archived_here: false,
            app_unlisted: false,
            resume_refusal: None,
        }
    }

    /// The rc8 walk: History read "Archived (0)" over a list of one
    /// archived in AI Juggler; the counts take it from the facet.
    #[test]
    fn one_archived_here_counts_as_archived_unless_its_open() {
        let mut shelved = conversation("c-1", "work", None);
        shelved.archived_here = true;
        let mut open = conversation("c-2", "work", None);
        open.archived_here = true;
        open.live = true;
        let index = ByRelation::of(
            [shelved, open, conversation("c-3", "work", None)],
            &DesktopApps::default(),
        );
        let archived: Vec<(&str, bool)> = index
            .facets()
            .iter()
            .map(|f| (f.id.as_str(), f.archived))
            .collect();
        assert_eq!(archived, [("c-1", true), ("c-2", false), ("c-3", false)]);
    }

    #[test]
    fn each_conversation_is_found_by_its_project_app_tool_and_folder() {
        let mut started_here = conversation("c-3", "work", Some("p-1"));
        started_here.session_id = Some("s-1".into());
        let desktop = DesktopApps {
            by_session: [(
                "claude-code/c-2".to_owned(),
                "app:claude-app:/p/Work".to_owned(),
            )]
            .into(),
            ..DesktopApps::default()
        };
        let index = ByRelation::of(
            [
                conversation("c-1", "work", Some("p-1")),
                conversation("c-2", "work", None),
                started_here,
            ],
            &desktop,
        );
        assert_eq!(index.facets().len(), 2, "AI Juggler's own are its sessions");
        let ids = |rows: Vec<&Facet>| rows.iter().map(|f| f.id.clone()).collect::<Vec<_>>();
        assert_eq!(ids(index.of_project("p-1").collect()), ["c-1"]);
        assert_eq!(ids(index.of_app("cli:work").collect()), ["c-1"]);
        assert_eq!(
            ids(index.of_app("app:claude-app:/p/Work").collect()),
            ["c-2"]
        );
        assert_eq!(ids(index.of_tool("claude-code").collect()), ["c-1", "c-2"]);
        assert_eq!(ids(index.of_folder("work").collect()), ["c-1", "c-2"]);
        assert_eq!(index.of_project("p-2").count(), 0);
        assert_eq!(index.facets()[0].last_ms, 5);
    }

    /// 0.4.3: one its desktop App's own records don't list is in no App;
    /// its project, folder and tool still have it (History lists it).
    #[test]
    fn one_its_desktop_apps_records_dont_list_is_in_no_app() {
        let profile = "app:claude-app:/p/Lab";
        let mut listed = conversation("c-1", "lab", Some("p-1"));
        listed.app = Some(profile.into());
        let mut gone = conversation("c-2", "lab", Some("p-1"));
        gone.app = Some(profile.into());
        gone.app_unlisted = true;
        let index = ByRelation::of([listed, gone], &DesktopApps::default());
        let ids = |rows: Vec<&Facet>| rows.iter().map(|f| f.id.clone()).collect::<Vec<_>>();
        assert_eq!(ids(index.of_app(profile).collect()), ["c-1"]);
        assert_eq!(index.of_app("cli:lab").count(), 0, "nor its CLI App");
        assert_eq!(index.facets()[1].app, None);
        assert_eq!(ids(index.of_project("p-1").collect()), ["c-1", "c-2"]);
        assert_eq!(ids(index.of_folder("lab").collect()), ["c-1", "c-2"]);
        assert_eq!(ids(index.of_tool("claude-code").collect()), ["c-1", "c-2"]);
    }
}
