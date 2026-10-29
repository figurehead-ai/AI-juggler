//! How a Claude account gets a skill or plugin that comes with a claude.ai
//! account, step by step (switch-setup-items.md §1, "Exact steps").
//!
//! claude.ai syncs what is on for an account into Claude Code, one way,
//! when one of that account's sessions starts. AI Juggler never copies an
//! Anthropic-provided skill (the license beside them forbids it, and a
//! copy would replace Anthropic's own sync): it says how to turn it on.
//!
//! Every step here rests on a help-centre page, named beside it and in
//! `docs/research/capabilities/manifest.json` (`skills-sync`), re-checked
//! on the manifest's 30-day cycle. The user guide's "Things the other
//! account doesn't have" says the same, in the same words.

use juggler_adapter::inventory::{GetIt, Need, Origin};

/// claude.ai's skills page, linked by
/// <https://support.claude.com/en/articles/12512180-use-skills-in-claude>.
pub const SKILLS_PAGE: &str = "https://claude.ai/customize/skills";

/// Step 1, for every item.
const SIGN_IN: &str = "Sign in to claude.ai as {account}.";

/// Skills need code execution (12512180: "Code execution and file
/// creation", newer layout "Create and edit files").
const CODE_EXECUTION: &str = "In Settings > Capabilities, turn on Code execution and file \
                              creation (it may be called Create and edit files).";

/// The last step: a profile syncs only while one of its sessions runs.
const START_A_SESSION: &str = "Start a session on {account} so it syncs.";

/// What happens after (skills.md: at session start, then about every 10
/// minutes in use).
const ARRIVES: &str = "It arrives when that session starts, or within about 10 minutes in one \
                       that's already running. Then choose Check again.";

/// Team and Enterprise (13119606-managing-skills-as-an-admin): an owner
/// allows Skills; Team has it on by default.
fn owner_may_allow() -> Need {
    Need::Admin(
        "On a Team or Enterprise plan, your organization's owner may need to allow Skills first."
            .to_owned(),
    )
}

/// How the account named `{account}` gets the synced skill `name`, by
/// where it came from and what its `compatibility` line says it needs.
#[must_use]
pub fn skill(name: &str, origin: &Origin, requires: &[String]) -> GetIt {
    let mut needs = feature_needs(name, requires);
    needs.push(owner_may_allow());
    let steps = match origin {
        Origin::Provided { .. } => vec![
            SIGN_IN.to_owned(),
            CODE_EXECUTION.to_owned(),
            format!(
                "In Customize > Skills, find {name} under From Anthropic & Partners and turn it on."
            ),
            START_A_SESSION.to_owned(),
        ],
        // Own, the organization's or shared: claude.ai tells them apart,
        // AI Juggler can't (§1), so both ways are named.
        Origin::Account => vec![
            SIGN_IN.to_owned(),
            format!(
                "In Customize > Skills, look for {name} under From your organization or Shared \
                 with you, and turn it on."
            ),
            "If it's a skill you made, choose + > Create skill > Upload a skill, and upload a \
             ZIP of its folder."
                .to_owned(),
            START_A_SESSION.to_owned(),
        ],
        Origin::Own | Origin::Unknown => vec![
            SIGN_IN.to_owned(),
            CODE_EXECUTION.to_owned(),
            format!("In Customize > Skills, find {name} and turn it on."),
            START_A_SESSION.to_owned(),
        ],
    };
    GetIt {
        steps,
        command: None,
        open_url: Some(SKILLS_PAGE.to_owned()),
        needs,
        then: Some(ARRIVES.to_owned()),
    }
}

/// How the account named `{account}` gets the synced plugin `name`
/// (claude.com/docs/plugins/overview: Customize > Plugins > Discover >
/// Add; Claude Code reads synced plugins once per start). No page address
/// is given: only the skills page's is verified (§1).
#[must_use]
pub fn plugin(name: &str) -> GetIt {
    GetIt {
        steps: vec![
            SIGN_IN.to_owned(),
            format!("In Customize > Plugins, choose Discover, find {name} and add it."),
            "Start a new session on {account}, or type /reload-plugins in one that's running."
                .to_owned(),
        ],
        command: None,
        open_url: None,
        needs: vec![Need::Admin(
            "If your organization turned Skills off on claude.ai, plugins don't sync either."
                .to_owned(),
        )],
        then: Some("Then choose Check again.".to_owned()),
    }
}

/// What a skill needs beyond the account, by its name first, then by its
/// `compatibility` line. Each rests on a help-centre article:
/// - computer use: 14128542 ("in beta for Pro and Max plans"), and
///   code.claude.com/docs/en/computer-use.md;
/// - the built-in browser: 16607400 (Cowork; Team and Enterprise "unless
///   an owner has turned it off");
/// - Claude in Chrome: 12012173 ("available on paid plans", the
///   extension connected);
/// - Research: 11088861 (paid plans, web search on).
fn feature_needs(name: &str, requires: &[String]) -> Vec<Need> {
    let says = |word: &str| {
        requires
            .iter()
            .any(|line| line.to_ascii_lowercase().contains(word))
    };
    if name == "computer-use" || says("computer use") {
        return vec![
            Need::Plan(
                "Needs a Pro or Max plan: computer use isn't offered on Team or Enterprise."
                    .to_owned(),
            ),
            Need::Feature(
                "Needs computer use turned on in the Claude desktop app (Settings > \
                 Capabilities), where the app offers it on this computer."
                    .to_owned(),
            ),
        ];
    }
    if name == "chrome-browser" || says("claude in chrome") {
        return vec![
            Need::Plan("Needs a paid plan (Pro, Max, Team or Enterprise).".to_owned()),
            Need::Feature(
                "Needs the Claude in Chrome extension, installed and connected.".to_owned(),
            ),
        ];
    }
    if name == "built-in-browser" || says("cowork") {
        return vec![
            Need::Feature("Needs Cowork in the Claude desktop app.".to_owned()),
            Need::Admin(
                "On a Team or Enterprise plan, an owner may have turned the built-in browser off."
                    .to_owned(),
            ),
        ];
    }
    if name == "deep-research" {
        return vec![
            Need::Plan("Needs a paid plan (Pro, Max, Team or Enterprise).".to_owned()),
            Need::Feature("Needs web search turned on.".to_owned()),
        ];
    }
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn provided() -> Origin {
        Origin::Provided {
            by: "Anthropic".to_owned(),
        }
    }

    #[test]
    fn a_provided_skill_is_turned_on_and_ends_with_a_session() {
        let get_it = skill("computer-use", &provided(), &[]);
        assert_eq!(get_it.steps.first().map(String::as_str), Some(SIGN_IN));
        assert_eq!(
            get_it.steps.last().map(String::as_str),
            Some(START_A_SESSION)
        );
        assert!(get_it.steps[2].contains("From Anthropic & Partners"));
        assert_eq!(get_it.open_url.as_deref(), Some(SKILLS_PAGE));
        assert!(
            get_it
                .needs
                .iter()
                .any(|need| matches!(need, Need::Plan(text) if text.contains("Pro or Max")))
        );
        let filled = get_it.for_account("Personal");
        assert!(filled.steps[0].ends_with("as Personal."));
        assert!(!format!("{filled:?}").contains("{account}"));
    }

    #[test]
    fn a_compatibility_line_names_the_feature() {
        let needs = feature_needs(
            "browse",
            &["Claude Code, when the Claude in Chrome extension is connected".to_owned()],
        );
        assert!(
            needs
                .iter()
                .any(|need| matches!(need, Need::Feature(text) if text.contains("Chrome")))
        );
        assert!(feature_needs("notes", &[]).is_empty());
    }

    #[test]
    fn an_account_skill_names_both_ways_and_never_says_yours() {
        let get_it = skill("team-style", &Origin::Account, &[]);
        let all = get_it.steps.join(" ");
        assert!(all.contains("From your organization"));
        assert!(all.contains("Upload a skill"));
        assert!(!all.contains("your own"));
    }
}
