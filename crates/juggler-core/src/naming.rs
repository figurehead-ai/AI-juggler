//! How a session is named when nothing named it: a git branch only when
//! it says what the work was ("fix/limits"), never one that says nothing
//! ("main"). One rule for every place a name is made (the window's
//! `tellingBranch` holds the same list).

/// Branches that name no piece of work.
const PLAIN: [&str; 8] = [
    "main",
    "master",
    "trunk",
    "develop",
    "development",
    "dev",
    "head",
    "default",
];

/// `branch` when it says what the work was; `None` for an empty one or
/// one that names no work.
#[must_use]
pub fn telling_branch(branch: Option<&str>) -> Option<&str> {
    branch.filter(|branch| {
        let lower = branch.trim().to_lowercase();
        !lower.is_empty() && !PLAIN.contains(&lower.as_str())
    })
}

#[cfg(test)]
mod tests {
    use super::telling_branch;

    #[test]
    fn a_branch_names_a_session_only_when_it_says_what_the_work_was() {
        assert_eq!(telling_branch(Some("fix/limits")), Some("fix/limits"));
        for plain in [
            "main",
            "Master",
            "trunk",
            "develop",
            "development",
            "dev",
            "HEAD",
            "default",
            "",
            "  ",
        ] {
            assert_eq!(telling_branch(Some(plain)), None, "{plain}");
        }
        assert_eq!(telling_branch(None), None);
    }
}
