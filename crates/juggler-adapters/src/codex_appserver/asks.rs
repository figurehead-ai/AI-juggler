//! app-server's approvals and questions as neutral asks, and the user's
//! answers as app-server's responses. Only the options Codex itself
//! offers are shown (D-28 as amended): "Allow for this session" is
//! `acceptForSession`, an "Always allow" rule is the execpolicy or
//! network amendment Codex proposed, never one AI Juggler makes up.

use juggler_adapter::AdapterError;
use juggler_adapter::runtime::events::bounded;
use juggler_adapter::runtime::{
    Approval, ApprovalKind, AskAnswer, AskId, AskOption, Choice, OptionEffect, Question,
    QuestionPart, RuntimeEvent,
};
use serde_json::{Value, json};

/// How an open request is answered.
#[derive(Clone, Debug, PartialEq)]
pub enum Answering {
    /// `item/commandExecution/requestApproval`, with what Codex proposed.
    Command {
        /// The proposed execpolicy amendment (a command prefix).
        amendment: Option<Value>,
        /// The proposed network policy amendments.
        network: Vec<Value>,
    },
    /// `item/fileChange/requestApproval`.
    FileChange,
    /// `item/permissions/requestApproval`, with what it asked for.
    Permissions {
        /// The requested profile (granted as asked, or not at all).
        requested: Value,
    },
    /// `item/tool/requestUserInput`.
    UserInput,
    /// `mcpServer/elicitation/request` (declined or cancelled here; a form
    /// is filled in the terminal).
    Elicitation,
}

fn text(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}

fn option(id: &str, label: &str, effect: OptionEffect) -> AskOption {
    AskOption {
        id: id.to_owned(),
        label: label.to_owned(),
        effect,
    }
}

fn denials(stop_label: &str) -> [AskOption; 2] {
    [
        option("decline", "Deny", OptionEffect::Deny),
        option("cancel", stop_label, OptionEffect::DenyAndStop),
    ]
}

/// An approval event for `id`.
fn approval(
    id: &AskId,
    params: &Value,
    kind: ApprovalKind,
    summary: &str,
    options: Vec<AskOption>,
) -> RuntimeEvent {
    RuntimeEvent::ApprovalRequested {
        approval: Approval {
            id: id.clone(),
            kind,
            summary: bounded(summary),
            reason: text(params, "reason").map(|r| bounded(&r)),
            options,
        },
    }
}

/// What a command approval runs, in the tool's own parsed words: every
/// `commandActions[].command`, joined. Codex 0.163.0 wraps `command` in the
/// user's shell (`/bin/zsh -lc ls`, lab 2, 2026-10-25), so that is only
/// the fallback.
fn actions_words(params: &Value) -> Option<String> {
    let commands: Vec<String> = params
        .get("commandActions")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|action| text(action, "command"))
        .filter(|command| !command.trim().is_empty())
        .collect();
    (!commands.is_empty()).then(|| commands.join(" · "))
}

/// `item/commandExecution/requestApproval`: Allow once, for the session,
/// the rules Codex proposed (a command prefix, network hosts), Deny.
fn command_ask(id: &AskId, params: &Value) -> (RuntimeEvent, Answering) {
    let amendment = params
        .get("proposedExecpolicyAmendment")
        .filter(|v| v.as_array().is_some_and(|a| !a.is_empty()))
        .cloned();
    let network: Vec<Value> = params
        .get("proposedNetworkPolicyAmendments")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let summary = actions_words(params)
        .or_else(|| text(params, "command"))
        .unwrap_or_else(|| {
            network.first().and_then(|n| text(n, "host")).map_or_else(
                || "Run a command".to_owned(),
                |host| format!("Reach {host}"),
            )
        });
    let mut options = vec![
        option("accept", "Allow once", OptionEffect::AllowOnce),
        option(
            "acceptForSession",
            "Allow for this session",
            OptionEffect::AllowRule {
                rule: "this command, until the session ends".into(),
            },
        ),
    ];
    if let Some(prefix) = amendment.as_ref().and_then(Value::as_array) {
        let words: Vec<&str> = prefix.iter().filter_map(Value::as_str).collect();
        let rule = format!("commands starting with `{}`", words.join(" "));
        options.push(option(
            "acceptWithExecpolicyAmendment",
            &format!("Always allow `{}`", words.join(" ")),
            OptionEffect::AllowRule { rule },
        ));
    }
    for (index, amendment) in network.iter().enumerate() {
        let host = text(amendment, "host").unwrap_or_default();
        let allow = text(amendment, "action").as_deref() != Some("deny");
        let (label, effect) = if allow {
            (
                format!("Always allow {host}"),
                OptionEffect::AllowRule {
                    rule: format!("network access to {host}"),
                },
            )
        } else {
            (format!("Always block {host}"), OptionEffect::Deny)
        };
        options.push(option(&format!("network:{index}"), &label, effect));
    }
    options.extend(denials("Deny and stop"));
    only_available(params, &mut options);
    (
        approval(id, params, ApprovalKind::Command, &summary, options),
        Answering::Command { amendment, network },
    )
}

/// Codex may say which decisions a request takes (`availableDecisions`,
/// seen live on 0.163.0 though not in its stable schema): then only those
/// are offered, never one it didn't list.
fn only_available(params: &Value, options: &mut Vec<AskOption>) {
    let Some(available) = params.get("availableDecisions").and_then(Value::as_array) else {
        return;
    };
    let names: Vec<String> = available
        .iter()
        .filter_map(|decision| {
            decision
                .as_str()
                .map(str::to_owned)
                .or_else(|| decision.as_object()?.keys().next().cloned())
        })
        .collect();
    options.retain(|option| {
        let name = if option.id.starts_with("network:") {
            "applyNetworkPolicyAmendment"
        } else {
            option.id.as_str()
        };
        names.iter().any(|known| known == name)
    });
}

/// `item/fileChange/requestApproval`.
fn file_ask(id: &AskId, params: &Value) -> (RuntimeEvent, Answering) {
    let summary = text(params, "grantRoot").map_or_else(
        || "Change files".to_owned(),
        |root| format!("Write files under {root}"),
    );
    let mut options = vec![
        option("accept", "Allow once", OptionEffect::AllowOnce),
        option(
            "acceptForSession",
            "Allow for this session",
            OptionEffect::AllowRule {
                rule: "changes to these files, until the session ends".into(),
            },
        ),
    ];
    options.extend(denials("Deny and stop"));
    (
        approval(id, params, ApprovalKind::FileChange, &summary, options),
        Answering::FileChange,
    )
}

/// `item/permissions/requestApproval`: granted as asked, for the turn or
/// the session, or not at all.
fn permissions_ask(id: &AskId, params: &Value) -> (RuntimeEvent, Answering) {
    let requested = params.get("permissions").cloned().unwrap_or(Value::Null);
    let mut wants = Vec::new();
    if requested.get("fileSystem").is_some_and(|v| !v.is_null()) {
        wants.push("more folders");
    }
    if requested.get("network").is_some_and(|v| !v.is_null()) {
        wants.push("the network");
    }
    let summary = if wants.is_empty() {
        "More permissions".to_owned()
    } else {
        format!("Access to {}", wants.join(" and "))
    };
    let options = vec![
        option("turn", "Allow for this turn", OptionEffect::AllowOnce),
        option(
            "session",
            "Allow for this session",
            OptionEffect::AllowRule {
                rule: "these permissions, until the session ends".into(),
            },
        ),
        option("decline", "Deny", OptionEffect::Deny),
    ];
    (
        approval(id, params, ApprovalKind::Permissions, &summary, options),
        Answering::Permissions { requested },
    )
}

/// `item/tool/requestUserInput`: each question with Codex's own choices.
fn question_ask(id: &AskId, params: &Value) -> (RuntimeEvent, Answering) {
    let parts = params
        .get("questions")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|q| QuestionPart {
            id: text(q, "id").unwrap_or_default(),
            header: bounded(&text(q, "header").unwrap_or_default()),
            question: bounded(&text(q, "question").unwrap_or_default()),
            choices: q
                .get("options")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .map(|o| Choice {
                    label: text(o, "label").unwrap_or_default(),
                    description: text(o, "description"),
                })
                .collect(),
            multi: false,
            free_text: q.get("isOther").and_then(Value::as_bool).unwrap_or(false)
                || q.get("options").is_none_or(Value::is_null),
            secret: q.get("isSecret").and_then(Value::as_bool).unwrap_or(false),
        })
        .collect();
    (
        RuntimeEvent::QuestionAsked {
            question: Question {
                id: id.clone(),
                parts,
            },
        },
        Answering::UserInput,
    )
}

/// A server request as an ask: its event and how it is answered; `None`
/// for a method that isn't an ask.
#[must_use]
pub fn ask_of(id: &AskId, method: &str, params: &Value) -> Option<(RuntimeEvent, Answering)> {
    match method {
        "item/commandExecution/requestApproval" => Some(command_ask(id, params)),
        "item/fileChange/requestApproval" => Some(file_ask(id, params)),
        "item/permissions/requestApproval" => Some(permissions_ask(id, params)),
        "item/tool/requestUserInput" => Some(question_ask(id, params)),
        "mcpServer/elicitation/request" => {
            let server = text(params, "serverName").unwrap_or_else(|| "A tool".into());
            let message = text(params, "message").unwrap_or_default();
            let options = vec![
                option("decline", "Decline", OptionEffect::Deny),
                option("cancel", "Cancel", OptionEffect::DenyAndStop),
            ];
            let summary = format!("{server} asks: {message}");
            Some((
                approval(id, params, ApprovalKind::Tool, &summary, options),
                Answering::Elicitation,
            ))
        }
        _ => None,
    }
}

/// The response the user's answer makes (already checked against the open
/// ask: an option it offered, or a reply to its parts).
///
/// # Errors
///
/// [`AdapterError::InvalidLaunch`] when the answer doesn't fit.
pub fn response(how: &Answering, answer: &AskAnswer) -> Result<Value, AdapterError> {
    let unfit = || AdapterError::InvalidLaunch("that answer doesn't fit this ask".into());
    match (how, answer) {
        (Answering::Command { amendment, network }, AskAnswer::Choose { option }) => {
            let decision = match option.as_str() {
                "accept" | "acceptForSession" | "decline" | "cancel" => json!(option),
                "acceptWithExecpolicyAmendment" => json!({"acceptWithExecpolicyAmendment": {
                    "execpolicy_amendment": amendment.clone().ok_or_else(unfit)?}}),
                other => {
                    let index: usize = other
                        .strip_prefix("network:")
                        .and_then(|n| n.parse().ok())
                        .ok_or_else(unfit)?;
                    let amendment = network.get(index).cloned().ok_or_else(unfit)?;
                    json!({"applyNetworkPolicyAmendment": {"network_policy_amendment": amendment}})
                }
            };
            Ok(json!({ "decision": decision }))
        }
        (Answering::FileChange, AskAnswer::Choose { option }) => Ok(json!({ "decision": option })),
        (Answering::Permissions { requested }, AskAnswer::Choose { option }) => {
            Ok(match option.as_str() {
                "turn" | "session" => json!({"permissions": requested, "scope": option}),
                _ => json!({"permissions": {}, "scope": "turn"}),
            })
        }
        (Answering::UserInput, AskAnswer::Reply { parts }) => {
            let answers: serde_json::Map<String, Value> = parts
                .iter()
                .map(|part| (part.part.clone(), json!({ "answers": part.answers })))
                .collect();
            Ok(json!({ "answers": answers }))
        }
        (Answering::Elicitation, AskAnswer::Choose { option }) => Ok(json!({ "action": option })),
        _ => Err(unfit()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use juggler_adapter::runtime::PartAnswer;

    fn id() -> AskId {
        AskId("7".into())
    }

    fn options(event: &RuntimeEvent) -> Vec<String> {
        match event {
            RuntimeEvent::ApprovalRequested { approval } => {
                approval.options.iter().map(|o| o.id.clone()).collect()
            }
            _ => Vec::new(),
        }
    }

    #[test]
    fn a_command_offers_only_what_codex_proposed() {
        let plain = json!({"threadId": "t", "turnId": "u", "itemId": "i", "startedAtMs": 1, "command": "npm test"});
        let (event, how) = ask_of(&id(), "item/commandExecution/requestApproval", &plain).unwrap();
        assert_eq!(
            options(&event),
            ["accept", "acceptForSession", "decline", "cancel"]
        );
        assert!(
            response(
                &how,
                &AskAnswer::Choose {
                    option: "acceptWithExecpolicyAmendment".into()
                }
            )
            .is_err(),
            "no amendment was proposed"
        );
        let proposed = json!({"threadId": "t", "turnId": "u", "itemId": "i", "startedAtMs": 1,
            "command": "npm test", "proposedExecpolicyAmendment": ["npm", "test"],
            "proposedNetworkPolicyAmendments": [{"host": "registry.npmjs.org", "action": "allow"}]});
        let (event, how) =
            ask_of(&id(), "item/commandExecution/requestApproval", &proposed).unwrap();
        let RuntimeEvent::ApprovalRequested { approval } = &event else {
            unreachable!()
        };
        assert_eq!(approval.summary, "npm test");
        assert!(
            approval
                .options
                .iter()
                .any(|o| o.label == "Always allow `npm test`")
        );
        assert_eq!(
            response(
                &how,
                &AskAnswer::Choose {
                    option: "acceptWithExecpolicyAmendment".into()
                }
            )
            .unwrap(),
            json!({"decision": {"acceptWithExecpolicyAmendment": {"execpolicy_amendment": ["npm", "test"]}}})
        );
        assert_eq!(
            response(
                &how,
                &AskAnswer::Choose {
                    option: "network:0".into()
                }
            )
            .unwrap(),
            json!({"decision": {"applyNetworkPolicyAmendment": {"network_policy_amendment":
                {"host": "registry.npmjs.org", "action": "allow"}}}})
        );
        assert_eq!(
            response(
                &how,
                &AskAnswer::Choose {
                    option: "cancel".into()
                }
            )
            .unwrap(),
            json!({"decision": "cancel"})
        );
    }

    #[test]
    fn only_the_decisions_codex_lists_are_offered() {
        let params = json!({"threadId": "t", "turnId": "u", "itemId": "i", "startedAtMs": 1,
            "command": "ls", "proposedExecpolicyAmendment": ["ls"],
            "availableDecisions": ["accept", {"acceptWithExecpolicyAmendment": {"execpolicy_amendment": ["ls"]}}, "cancel"]});
        let (event, _) = ask_of(&id(), "item/commandExecution/requestApproval", &params).unwrap();
        assert_eq!(
            options(&event),
            ["accept", "acceptWithExecpolicyAmendment", "cancel"]
        );
    }

    #[test]
    fn questions_and_permissions_answer_in_codexs_shapes() {
        let params = json!({"threadId": "t", "turnId": "u", "itemId": "i", "isBlocking": true,
            "questions": [{"id": "db", "header": "Database", "question": "Which one?",
                "options": [{"label": "Postgres", "description": "SQL"}, {"label": "SQLite", "description": "file"}]}]});
        let (event, how) = ask_of(&id(), "item/tool/requestUserInput", &params).unwrap();
        let RuntimeEvent::QuestionAsked { question } = &event else {
            unreachable!()
        };
        assert_eq!(question.parts[0].choices[0].label, "Postgres");
        assert!(!question.parts[0].free_text);
        let reply = AskAnswer::Reply {
            parts: vec![PartAnswer {
                part: "db".into(),
                answers: vec!["Postgres".into()],
            }],
        };
        assert_eq!(
            response(&how, &reply).unwrap(),
            json!({"answers": {"db": {"answers": ["Postgres"]}}})
        );
        let perms = json!({"threadId": "t", "turnId": "u", "itemId": "i", "startedAtMs": 1, "cwd": "/w",
            "permissions": {"network": {"enabled": true}}});
        let (event, how) = ask_of(&id(), "item/permissions/requestApproval", &perms).unwrap();
        assert_eq!(options(&event), ["turn", "session", "decline"]);
        assert_eq!(
            response(
                &how,
                &AskAnswer::Choose {
                    option: "session".into()
                }
            )
            .unwrap(),
            json!({"permissions": {"network": {"enabled": true}}, "scope": "session"})
        );
        assert_eq!(
            response(
                &how,
                &AskAnswer::Choose {
                    option: "decline".into()
                }
            )
            .unwrap(),
            json!({"permissions": {}, "scope": "turn"})
        );
        assert!(ask_of(&id(), "item/tool/call", &json!({})).is_none());
    }
}
