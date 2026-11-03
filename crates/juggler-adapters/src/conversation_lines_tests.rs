use super::*;

fn encode(bytes: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let n = chunk
            .iter()
            .enumerate()
            .fold(0u32, |n, (i, b)| n | u32::from(*b) << (16 - 8 * i));
        for i in 0..=chunk.len() {
            out.push(ALPHABET[(n >> (18 - 6 * i) & 63) as usize] as char);
        }
    }
    while !out.len().is_multiple_of(4) {
        out.push('=');
    }
    out
}

/// A signature whose field 2 → 1 → 8 holds `mark`.
fn signature(mark: &str) -> String {
    let mut inner = vec![0x42, u8::try_from(mark.len()).unwrap()];
    inner.extend_from_slice(mark.as_bytes());
    let mut middle = vec![0x0a, u8::try_from(inner.len()).unwrap()];
    middle.extend(inner);
    let mut outer = vec![0x08, 0x01, 0x12, u8::try_from(middle.len()).unwrap()];
    outer.extend(middle);
    encode(&outer)
}

#[test]
fn a_claude_reply_with_thinking_words_and_a_described_command() {
    let line = serde_json::json!({
        "type": "assistant", "uuid": "u1", "timestamp": "2026-10-24T10:00:00Z",
        "message": {"id": "msg_1", "role": "assistant", "model": "m1", "content": [
            {"type": "thinking", "thinking": " Let me look. ", "signature": signature("narration")},
            {"type": "text", "text": " Running it. "},
            {"type": "tool_use", "id": "t", "name": "Bash", "input": {"command": "pnpm test", "description": "Run the tests"}},
        ]},
    });
    let Said::Message(message) = claude(&line) else {
        unreachable!("a message")
    };
    assert_eq!(message.speaker, Speaker::Assistant);
    assert_eq!(message.text(), "Running it.");
    assert_eq!(
        message.parts[0],
        Part::Thought {
            text: "Let me look.".to_owned(),
            narration: true
        }
    );
    assert_eq!(message.message_id.as_deref(), Some("msg_1"));
    assert_eq!(message.uuid.as_deref(), Some("u1"));
    assert_eq!(message.model.as_deref(), Some("m1"));
    let Part::Call(call) = &message.parts[2] else {
        unreachable!("a call")
    };
    assert_eq!(call.tool, "Bash");
    assert_eq!(call.call_id.as_deref(), Some("t"));
    assert_eq!(call.line, "ran \"pnpm test\"");
    assert_eq!(call.input.description.as_deref(), Some("Run the tests"));
    assert_eq!(call.input.command.as_deref(), Some("pnpm test"));
    assert!(message.at_ms.is_some());
}

#[test]
fn only_a_narration_signature_marks_thinking_as_narration() {
    assert!(is_narration(&signature("narration")));
    assert!(!is_narration(&signature("summary")));
    assert!(!is_narration("not base64 at all!"));
    assert!(!is_narration(&encode(&[0x12, 0x40, 0x01])), "cut short");
    assert!(!is_narration(""));
}

#[test]
fn a_result_says_how_its_call_ended() {
    let line = serde_json::json!({
        "type": "user", "toolUseResult": {"type": "create", "filePath": "/w/a.ts"},
        "message": {"role": "user", "content": [{"type": "tool_result", "tool_use_id": "t1", "content": "ok"}]},
    });
    assert_eq!(
        claude(&line),
        Said::Results {
            results: vec![CallResult {
                call_id: "t1".to_owned(),
                failed: false,
                kind: Some("create".to_owned())
            }],
            sidechain: false
        }
    );
    let failed = serde_json::json!({
        "type": "user", "toolUseResult": "Error: no such file",
        "message": {"role": "user", "content": [{"type": "tool_result", "tool_use_id": "t2", "is_error": true, "content": "no"}]},
    });
    let Said::Results { results, .. } = claude(&failed) else {
        unreachable!("results")
    };
    assert!(results[0].failed);
    assert_eq!(results[0].kind, None);
}

#[test]
fn meta_lines_progress_and_other_kinds_say_nothing() {
    for line in [
        serde_json::json!({"type": "user", "isMeta": true, "message": {"role": "user", "content": "Caveat"}}),
        serde_json::json!({"type": "progress", "data": {"message": {"role": "assistant", "content": "x"}}}),
        serde_json::json!({"type": "system", "subtype": "compact_boundary"}),
        serde_json::json!({"type": "user", "message": {"role": "user", "content": "<system-reminder>be brief</system-reminder>\n"}}),
    ] {
        assert_eq!(claude(&line), Said::Nothing, "{line}");
    }
}

#[test]
fn a_persons_words_lose_the_tools_reminders() {
    let line = serde_json::json!({"type": "user", "message": {"role": "user", "content": [
        {"type": "text", "text": "Fix the header"},
        {"type": "text", "text": "<system-reminder>The date is today.</system-reminder>"},
    ]}});
    let Said::Message(message) = claude(&line) else {
        unreachable!("a message")
    };
    assert_eq!(message.text(), "Fix the header");
}

#[test]
fn a_prompt_the_person_queued_mid_turn_is_theirs_a_notification_is_not() {
    let queued = |kind: &str, mode: &str, prompt: Value| {
        serde_json::json!({
            "type": "attachment", "uuid": "q1", "timestamp": "2026-10-24T10:00:00Z",
            "attachment": {"type": "queued_command", "origin": {"kind": kind}, "commandMode": mode, "prompt": prompt},
        })
    };
    let Said::Message(message) = claude(&queued(
        "human",
        "prompt",
        Value::from("also check the footer"),
    )) else {
        unreachable!("the person's message")
    };
    assert_eq!(message.speaker, Speaker::You);
    assert_eq!(message.text(), "also check the footer");
    let blocks = serde_json::json!([{"type": "text", "text": "and the tests"}]);
    let Said::Message(message) = claude(&queued("human", "prompt", blocks)) else {
        unreachable!("the person's message")
    };
    assert_eq!(message.text(), "and the tests");
    assert_eq!(
        claude(&queued(
            "task-notification",
            "prompt",
            Value::from("<task-notification/>")
        )),
        Said::Nothing
    );
    assert_eq!(
        claude(&queued("human", "bash", Value::from("ls"))),
        Said::Nothing
    );
}

#[test]
fn a_rollouts_lines_by_its_own_rules() {
    let item = |payload: Value| serde_json::json!({"timestamp": "2026-10-24T10:00:00Z", "type": "response_item", "payload": payload});
    assert_eq!(
        codex(
            &serde_json::json!({"timestamp": "t", "type": "turn_context", "payload": {"model": "gpt-5"}}),
            None
        ),
        Said::Model("gpt-5".to_owned())
    );
    let context = item(
        serde_json::json!({"type": "message", "role": "user", "content": [{"type": "input_text", "text": "<environment_context>cwd</environment_context>"}]}),
    );
    assert_eq!(codex(&context, None), Said::Nothing);
    let Said::Message(call) = codex(
        &item(
            serde_json::json!({"type": "function_call", "name": "shell", "call_id": "c1", "arguments": "{\"command\":[\"make\",\"test\"]}"}),
        ),
        Some("gpt-5"),
    ) else {
        unreachable!("a call")
    };
    assert_eq!(call.speaker, Speaker::Assistant);
    let Part::Call(call) = &call.parts[0] else {
        unreachable!("a call")
    };
    assert_eq!(call.tool, "shell");
    assert_eq!(call.input.command.as_deref(), Some("make test"));
    let Said::Message(reply) = codex(
        &item(
            serde_json::json!({"type": "message", "id": "m9", "role": "assistant", "content": [{"type": "output_text", "text": "Done."}]}),
        ),
        Some("gpt-5"),
    ) else {
        unreachable!("a reply")
    };
    assert_eq!(reply.text(), "Done.");
    assert_eq!(reply.model.as_deref(), Some("gpt-5"));
    assert_eq!(reply.message_id.as_deref(), Some("m9"));
}

#[test]
fn a_message_whose_image_data_was_left_out_keeps_its_words() {
    // As the feed holds them (`inline_data`): the part stays, its bytes
    // gone.
    let line = serde_json::json!({
        "type": "user", "timestamp": "2026-10-24T10:00:00Z",
        "message": {"role": "user", "content": [
            {"type": "text", "text": "look at this"},
            {"type": "image", "source": {"type": "base64", "media_type": "image/png", "data": ""}},
        ]},
    });
    let Said::Message(message) = claude(&line) else {
        unreachable!("a message")
    };
    assert_eq!(message.speaker, Speaker::You);
    assert_eq!(message.text(), "look at this");
    let line = serde_json::json!({
        "timestamp": "2026-10-24T10:00:00Z", "type": "response_item",
        "payload": {"type": "message", "role": "user", "content": [
            {"type": "input_text", "text": "look at this"},
            {"type": "input_image", "image_url": "data:image/png;base64,"},
        ]},
    });
    let Said::Message(message) = codex(&line, None) else {
        unreachable!("a message")
    };
    assert_eq!(message.text(), "look at this");
}
