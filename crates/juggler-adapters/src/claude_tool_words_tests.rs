//! The Claude app's words for one call, as its Code tab writes them.

use super::*;

fn call(action: Option<&str>) -> CallInput {
    CallInput {
        action: action.map(str::to_owned),
        ..CallInput::default()
    }
}

fn batch(steps: &[(&str, &str)]) -> CallInput {
    CallInput {
        batch: steps
            .iter()
            .map(|(name, action)| ((*name).to_owned(), (*action).to_owned()))
            .collect(),
        ..CallInput::default()
    }
}

#[test]
fn the_in_app_browser_by_its_action() {
    let tool = "mcp__Claude_Browser__computer";
    assert_eq!(
        words(tool, &call(Some("screenshot")), false),
        "Captured page"
    );
    assert_eq!(
        words(tool, &call(Some("screenshot")), true),
        "Capturing page"
    );
    assert_eq!(words(tool, &call(Some("left_click")), false), "Clicked");
    // An action it has no words for: the tool's own.
    assert_eq!(
        words(tool, &call(Some("dance")), false),
        "Interacted with page"
    );
    assert_eq!(
        words("mcp__Claude_Browser__find", &call(None), false),
        "Found on page"
    );
}

#[test]
fn a_batch_names_its_first_three_different_actions() {
    let tool = "mcp__Claude_Browser__browser_batch";
    let steps = batch(&[
        ("find", ""),
        ("computer", "left_click"),
        ("computer", "left_click"),
        ("computer", "type"),
        ("computer", "screenshot"),
        ("navigate", ""),
    ]);
    assert_eq!(words(tool, &steps, false), "Found, clicked, typed +2");
    assert_eq!(words(tool, &batch(&[]), false), "Performed browser actions");
}

#[test]
fn a_page_opened_by_its_address_alone() {
    let tool = "mcp__Claude_Browser__preview_start";
    let by_url = CallInput {
        url_without_name: true,
        ..CallInput::default()
    };
    assert_eq!(words(tool, &by_url, false), "Opened a page");
    assert_eq!(words(tool, &call(None), false), "Started preview");
}

#[test]
fn claude_in_chrome_in_the_browsers_words() {
    assert_eq!(
        words("mcp__claude-in-chrome__navigate", &call(None), false),
        "Opened a page"
    );
    assert_eq!(
        words(
            "mcp__claude-in-chrome__tabs_context_mcp",
            &call(None),
            false
        ),
        "Checked tabs"
    );
    assert_eq!(
        words(
            "mcp__claude-in-chrome__computer",
            &call(Some("scroll")),
            false
        ),
        "Scrolled"
    );
}

#[test]
fn the_apps_own_session_tools() {
    assert_eq!(
        words("mcp__ccd_turn__update_plan", &call(None), false),
        "Updated plan"
    );
    assert_eq!(
        words("mcp__ccd_turn__update_plan", &call(None), true),
        "Updating plan"
    );
}

#[test]
fn a_simulator_and_computer_use() {
    let control = "mcp__Claude_Code_iOS_Simulator__control";
    assert_eq!(
        words(control, &call(Some("screenshot")), false),
        "Captured screen"
    );
    assert_eq!(
        words(control, &call(Some("dance")), false),
        "Used the device"
    );
    assert_eq!(
        words(
            "mcp__Claude_Code_iOS_Simulator__build_1a2b3c4d",
            &call(None),
            false
        ),
        "Built app"
    );
    let steps = batch(&[("", "left_click"), ("", "type")]);
    assert_eq!(
        words("mcp__computer-use__computer_batch", &steps, false),
        "Clicked, typed"
    );
}

#[test]
fn built_in_tools_by_kind() {
    assert_eq!(words("ToolSearch", &call(None), false), "Loaded tools");
    assert_eq!(words("ToolSearch", &call(None), true), "Loading tools");
    assert_eq!(words("ExitPlanMode", &call(None), false), "Proposed a plan");
}

#[test]
fn any_other_tool_by_its_server_and_name_never_raw() {
    assert_eq!(
        words("mcp__visualize__show_widget", &call(None), false),
        "Used visualize: Show widget"
    );
    assert_eq!(
        words("mcp__visualize__show_widget", &call(None), true),
        "Using visualize: Show widget"
    );
    assert_eq!(
        words("mcp__my_server__getGitHubIssue", &call(None), false),
        "Used my server: Get git hub issue"
    );
    assert_eq!(
        words("SomeNewTool", &call(None), false),
        "Used Some new tool"
    );
}

#[test]
fn every_table_is_sorted_for_its_search() {
    let sorted = |keys: Vec<&str>| keys.windows(2).all(|pair| pair[0] < pair[1]);
    for table in [
        table::MCP,
        table::POINTER,
        table::COMPUTER,
        table::PAGE,
        table::DEVICE,
        table::DEVICE_OTHER,
    ] {
        assert!(sorted(table.iter().map(|(key, _, _)| *key).collect()));
    }
    for table in [table::CHROME, table::BUILTIN_DONE, table::BUILTIN_RUNNING] {
        assert!(sorted(table.iter().map(|(key, _)| *key).collect()));
    }
}
