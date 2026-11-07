//! L1 (`docs/qa/0.4-validation-plan.md` §3; AJ-E3302): how folders group
//! into Accounts, enumerated in full and checked against the reference
//! model (`scripts/qa/account_oracle.py`), which is written from D-62 and
//! D-35 rather than from this code.
//!
//! Three folders, each on a tool (Claude Code or Codex), signed in as A, as
//! A in a team, as B, unreadable or not signed in, with a plan of none, Pro
//! or Max: (2 × 5 × 3)³ = 27 000 cases. For each, the Accounts in order,
//! their state, their folders and their plan (or that it differs).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

use juggler_adapter::IdentityProbe;
use juggler_app::features::subscriptions::{ProfileRead, SubscriptionProfileInfo, group};

const TOOLS: [&str; 2] = ["claude-code", "codex"];
const PROBES: [&str; 5] = ["in-a", "in-a-team", "in-b", "unreadable", "out"];
const PLANS: [&str; 3] = ["none", "pro", "max"];

/// The oracle's table `name`, by case.
fn oracle(name: &str) -> HashMap<String, String> {
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/qa/account_oracle.py");
    let out = Command::new("python3")
        .arg(&script)
        .arg(name)
        .output()
        .expect("python3 runs the reference model");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout)
        .expect("utf-8")
        .lines()
        .map(|line| {
            let (case, expected) = line.split_once('\t').expect("a tab");
            (case.to_owned(), expected.to_owned())
        })
        .collect()
}

fn probe(kind: &str) -> IdentityProbe {
    let signed_in = |subject: &str, label: &str, org: Option<&str>| IdentityProbe::SignedIn {
        subject: subject.to_owned(),
        label: label.to_owned(),
        org: org.map(str::to_owned),
    };
    match kind {
        "in-a" => signed_in("subject-a", "a…@example.com", None),
        "in-a-team" => signed_in("subject-a", "a…@example.com · Acme", Some("org-acme")),
        "in-b" => signed_in("subject-b", "b…@example.com", None),
        "unreadable" => IdentityProbe::Unverifiable {
            reason: "its state file couldn't be read".to_owned(),
        },
        _ => IdentityProbe::SignedOut,
    }
}

fn read(at: usize, tool: &str, kind: &str, plan: &str) -> ProfileRead {
    ProfileRead {
        profile: SubscriptionProfileInfo {
            id: format!("p{at}"),
            label: format!("Folder {at}"),
            folder: format!("~/.folder-{at}"),
            plan: (plan != "none").then(|| plan.to_owned()),
            renewal_date: None,
            fee_usd: None,
        },
        tool: tool.to_owned(),
        probe: probe(kind),
        service: None,
    }
}

#[test]
fn folders_group_into_accounts_as_the_reference_model_says() {
    let want = oracle("group");
    let one: Vec<(&str, &str, &str)> = TOOLS
        .iter()
        .flat_map(|tool| {
            PROBES
                .iter()
                .flat_map(move |probe| PLANS.iter().map(move |plan| (*tool, *probe, *plan)))
        })
        .collect();
    let mut checked = 0usize;
    let mut wrong = Vec::new();
    let mut disagree = 0usize;
    for a in &one {
        for b in &one {
            for c in &one {
                let folders = [a, b, c];
                let case = folders
                    .iter()
                    .map(|(tool, probe, plan)| format!("{tool}/{probe}/{plan}"))
                    .collect::<Vec<_>>()
                    .join(";");
                let rows = folders
                    .iter()
                    .enumerate()
                    .map(|(at, (tool, probe, plan))| read(at + 1, tool, probe, plan))
                    .collect();
                let got = group(rows)
                    .iter()
                    .map(|account| {
                        let ids = account
                            .profiles
                            .iter()
                            .map(|p| p.id.as_str())
                            .collect::<Vec<_>>()
                            .join("+");
                        let plan = if account.differs.iter().any(|term| term == "plan") {
                            "differs".to_owned()
                        } else {
                            account.plan.clone().unwrap_or_else(|| "none".to_owned())
                        };
                        format!("{}:{ids}:{plan}", account.state)
                    })
                    .collect::<Vec<_>>()
                    .join(" | ");
                let expected = want
                    .get(&case)
                    .unwrap_or_else(|| panic!("the oracle has no case {case}"));
                if &got != expected {
                    disagree += 1;
                }
                if &got != expected && wrong.len() < 10 {
                    wrong.push(format!("{case}\n  got  {got}\n  want {expected}"));
                }
                checked += 1;
            }
        }
    }
    assert_eq!(
        checked,
        want.len(),
        "every case the oracle lists is checked"
    );
    assert!(
        disagree == 0,
        "{} of {checked} disagree, first ones:\n{}",
        disagree,
        wrong.join("\n")
    );
}
