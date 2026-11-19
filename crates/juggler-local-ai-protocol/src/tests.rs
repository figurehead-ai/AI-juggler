use super::*;

const SHA: &str = "061b54daade076b5d3362dac252678d17da8c68f07560be70818cace6590cb1a";

fn generate() -> Request {
    Request::Generate {
        id: 7,
        instructions: "Say one line.".to_owned(),
        prompt: "Message to summarise".to_owned(),
        guide: Some(Guide::JsonSchema(r#"{"type":"string"}"#.to_owned())),
        sampling: Sampling {
            temperature_milli: Some(700),
            top_p_milli: Some(800),
            top_k: Some(20),
            min_p_milli: Some(0),
            presence_milli: Some(1_500),
        },
        max_tokens: 60,
        timeout_ms: 10_000,
    }
}

fn load() -> Request {
    Request::Load {
        id: 1,
        file: model_file_name(SHA).unwrap(),
        size_bytes: 1_834_426_016,
        device: 16_777_234,
        inode: 4_242,
        context_tokens: 4_096,
    }
}

#[test]
fn every_request_round_trips_through_its_line() {
    for request in [
        load(),
        generate(),
        Request::Cancel { id: 7 },
        Request::Quit {},
    ] {
        let line = request.encode().unwrap();
        assert_eq!(line.last(), Some(&b'\n'));
        assert_eq!(parse_request(&line[..line.len() - 1]).unwrap(), request);
    }
}

#[test]
fn every_answer_round_trips_through_its_line() {
    for answer in [
        Answer::Hello {
            protocol: PROTOCOL,
            gpu: true,
            sandboxed: true,
        },
        Answer::Loaded {
            id: 1,
            context_tokens: 4_096,
        },
        Answer::Text {
            id: 7,
            text: "One line.".to_owned(),
            finish: Finish::Length,
            tokens: 3,
        },
        Answer::Error {
            id: None,
            error: Fault::BadRequest,
        },
    ] {
        let line = answer.encode().unwrap();
        assert_eq!(parse_answer(&line[..line.len() - 1]).unwrap(), answer);
    }
}

#[test]
fn malformed_lines_are_bad_requests() {
    for line in [
        &b""[..],
        b"not json",
        b"[]",
        b"{}",
        b"{\"op\":\"shell\",\"id\":1}",
        b"{\"op\":\"cancel\"}",
        b"{\"op\":\"cancel\",\"id\":-1}",
        b"{\"op\":\"cancel\",\"id\":1.5}",
        b"{\"op\":\"cancel\",\"id\":\"1\"}",
        // An unknown field, and a duplicate one.
        b"{\"op\":\"cancel\",\"id\":1,\"path\":\"/etc/passwd\"}",
        b"{\"op\":\"cancel\",\"id\":1,\"id\":2}",
        b"{\"op\":\"quit\",\"now\":true}",
        b"\xff\xfe",
    ] {
        assert_eq!(
            parse_request(line),
            Err(Fault::BadRequest),
            "{}",
            String::from_utf8_lossy(line)
        );
    }
}

#[test]
fn an_oversized_line_is_refused_before_it_is_read() {
    let mut line = br#"{"op":"cancel","id":1}"#.to_vec();
    line.resize(MAX_REQUEST_BYTES + 1, b' ');
    assert_eq!(parse_request(&line), Err(Fault::TooLarge));
    let Request::Generate {
        id,
        instructions,
        guide,
        sampling,
        max_tokens,
        timeout_ms,
        ..
    } = generate()
    else {
        unreachable!()
    };
    let big = Request::Generate {
        id,
        instructions,
        prompt: "a".repeat(MAX_REQUEST_BYTES),
        guide,
        sampling,
        max_tokens,
        timeout_ms,
    };
    assert_eq!(big.encode(), Err(Fault::TooLarge));
    let mut answer = vec![b' '; MAX_ANSWER_BYTES + 1];
    answer[0] = b'{';
    assert_eq!(parse_answer(&answer), Err(Fault::TooLarge));
}

#[test]
fn a_file_is_named_only_by_its_hash() {
    assert!(is_model_file_name(&format!("{SHA}.gguf")));
    for name in [
        "../x.gguf".to_owned(),
        format!("/{SHA}.gguf"),
        format!("{SHA}.GGUF"),
        format!("{SHA}.gguf/.."),
        format!("{}.gguf", SHA.to_uppercase()),
        format!("{}.gguf", &SHA[..63]),
        format!("{SHA}0.gguf"),
        format!("{SHA}.json"),
        SHA.to_owned(),
        format!("../{SHA}.gguf"),
        format!("{SHA}.gguf\0"),
    ] {
        assert!(!is_model_file_name(&name), "{name}");
        let request = Request::Load {
            id: 1,
            file: name,
            size_bytes: 1,
            device: 1,
            inode: 1,
            context_tokens: 4_096,
        };
        assert_eq!(request.check(), Err(Fault::BadRequest));
    }
    assert_eq!(model_file_name("../etc"), None);
}

type Change = dyn Fn(&mut Sampling, &mut u32, &mut u64);

#[test]
fn numbers_outside_their_bounds_are_refused() {
    let with = |change: &Change| {
        let Request::Generate {
            id,
            instructions,
            prompt,
            guide,
            mut sampling,
            mut max_tokens,
            mut timeout_ms,
        } = generate()
        else {
            unreachable!()
        };
        change(&mut sampling, &mut max_tokens, &mut timeout_ms);
        Request::Generate {
            id,
            instructions,
            prompt,
            guide,
            sampling,
            max_tokens,
            timeout_ms,
        }
        .check()
    };
    assert_eq!(with(&|_, _, _| {}), Ok(()));
    let out: [&Change; 9] = [
        &|s, _, _| s.temperature_milli = Some(2_001),
        &|s, _, _| s.top_p_milli = Some(0),
        &|s, _, _| s.top_p_milli = Some(1_001),
        &|s, _, _| s.top_k = Some(201),
        &|s, _, _| s.min_p_milli = Some(1_001),
        &|s, _, _| s.presence_milli = Some(2_001),
        &|_, m, _| *m = 0,
        &|_, m, _| *m = 4_097,
        &|_, _, t| *t = 600_001,
    ];
    for change in out {
        assert_eq!(with(change), Err(Fault::BadRequest));
    }
    for (size, context) in [
        (0, 4_096),
        (8 * 1024 * 1024 * 1024 + 1, 4_096),
        (1, 1_023),
        (1, 32_769),
    ] {
        let request = Request::Load {
            id: 1,
            file: model_file_name(SHA).unwrap(),
            size_bytes: size,
            device: 1,
            inode: 1,
            context_tokens: context,
        };
        assert_eq!(request.check(), Err(Fault::BadRequest), "{size} {context}");
    }
}

#[test]
fn text_with_a_nul_and_oversized_or_empty_guides_are_refused() {
    let make = |instructions: &str, prompt: &str, guide: Option<Guide>| Request::Generate {
        id: 1,
        instructions: instructions.to_owned(),
        prompt: prompt.to_owned(),
        guide,
        sampling: Sampling::default(),
        max_tokens: 60,
        timeout_ms: 1_000,
    };
    assert_eq!(make("a\0", "b", None).check(), Err(Fault::BadRequest));
    assert_eq!(make("a", "b\0", None).check(), Err(Fault::BadRequest));
    assert_eq!(
        make("a", "b", Some(Guide::Grammar(String::new()))).check(),
        Err(Fault::BadRequest)
    );
    assert_eq!(
        make(
            "a",
            "b",
            Some(Guide::Grammar("x".repeat(MAX_GUIDE_BYTES + 1)))
        )
        .check(),
        Err(Fault::BadRequest)
    );
    assert_eq!(
        make("a", "b", Some(Guide::Grammar("root ::= \"\0\"".to_owned()))).check(),
        Err(Fault::BadRequest)
    );
    assert_eq!(make("a", "b", None).check(), Ok(()));
    assert!(Sampling::default().greedy());
}

#[test]
fn answers_are_strict_too() {
    for line in [
        &b"{\"kind\":\"text\",\"id\":1,\"text\":\"x\",\"finish\":\"stop\"}"[..],
        b"{\"kind\":\"text\",\"id\":1,\"text\":\"x\",\"finish\":\"maybe\",\"tokens\":1}",
        b"{\"kind\":\"error\",\"id\":1,\"error\":\"rm -rf\"}",
        b"{\"kind\":\"hello\",\"protocol\":3,\"gpu\":true,\"sandboxed\":true,\"extra\":1}",
        b"{\"kind\":\"shell\"}",
    ] {
        assert_eq!(parse_answer(line), Err(Fault::BadRequest));
    }
    let long = Answer::Text {
        id: 1,
        text: "a".repeat(MAX_TEXT_BYTES + 1),
        finish: Finish::Stop,
        tokens: 1,
    };
    let line = long.encode().unwrap();
    assert_eq!(parse_answer(&line[..line.len() - 1]), Err(Fault::TooLarge));
}

#[test]
fn the_most_text_fits_an_answer_even_fully_escaped() {
    let answer = Answer::Text {
        id: u64::MAX,
        text: "\u{1}".repeat(MAX_TEXT_BYTES),
        finish: Finish::Length,
        tokens: u32::MAX,
    };
    assert!(answer.encode().is_ok());
}

/// I2: parsing a line checks its bounds, not only its shape: a request
/// that reads as JSON but breaks a bound is refused by `parse_request`
/// itself.
#[test]
fn parse_request_checks_the_bounds() {
    for line in [
        r#"{"op":"generate","id":1,"instructions":"a","prompt":"b","max_tokens":0,"timeout_ms":1000}"#,
        r#"{"op":"generate","id":1,"instructions":"a","prompt":"b","max_tokens":10,"timeout_ms":1000,"sampling":{"top_k":201}}"#,
        r#"{"op":"generate","id":1,"instructions":"a\u0000","prompt":"b","max_tokens":10,"timeout_ms":1000}"#,
        r#"{"op":"load","id":1,"file":"../x.gguf","size_bytes":1,"device":1,"inode":1,"context_tokens":4096}"#,
        r#"{"op":"load","id":1,"file":"0000000000000000000000000000000000000000000000000000000000000000.gguf","size_bytes":0,"device":1,"inode":1,"context_tokens":4096}"#,
    ] {
        let request: Request = serde_json::from_str(line).unwrap();
        assert_eq!(request.check(), Err(Fault::BadRequest), "{line}");
        assert_eq!(
            parse_request(line.as_bytes()),
            Err(Fault::BadRequest),
            "{line}"
        );
    }
}

/// The profile's top-level forms, comments removed, whitespace collapsed.
fn forms(profile: &str) -> Vec<String> {
    let code: String = profile
        .lines()
        .map(|line| line.split(';').next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join(" ");
    let mut out = Vec::new();
    let (mut depth, mut start) = (0_i32, 0);
    for (i, c) in code.char_indices() {
        match c {
            '(' => {
                if depth == 0 {
                    start = i;
                }
                depth += 1;
            }
            ')' => {
                depth -= 1;
                if depth == 0 {
                    out.push(
                        code[start..=i]
                            .split_whitespace()
                            .collect::<Vec<_>>()
                            .join(" "),
                    );
                }
            }
            _ => {}
        }
    }
    assert_eq!(depth, 0, "balanced");
    out
}

#[test]
fn the_sandbox_denies_by_default_and_allows_one_service() {
    let forms = forms(SANDBOX_PROFILE);
    assert_eq!(forms[0], "(version 1)");
    assert_eq!(forms[1], "(deny default)");
    assert!(
        forms[2..]
            .iter()
            .all(|f| f.starts_with("(allow ") || f == "(deny process-info-listpids)"),
        "only allows after the default deny"
    );
    let lookups: Vec<&String> = forms.iter().filter(|f| f.contains("mach-lookup")).collect();
    assert_eq!(
        lookups,
        [&"(allow mach-lookup (global-name \"com.apple.MTLCompilerService\") (xpc-service-name \"com.apple.MTLCompilerService\"))".to_owned()]
    );
    let execs: Vec<&String> = forms
        .iter()
        .filter(|f| f.contains("process-exec"))
        .collect();
    assert_eq!(
        execs,
        [
            &"(allow process-exec (literal (param \"HELPER\")) (literal \"/usr/sbin/taskpolicy\"))"
                .to_owned()
        ]
    );
    let writes: Vec<&String> = forms.iter().filter(|f| f.contains("write")).collect();
    assert_eq!(
        writes,
        [&"(allow file-read* file-write* (subpath (param \"CACHE\")))".to_owned()]
    );
    for never in [
        "network",
        "process-fork",
        "lsopen",
        "launchservices",
        "/private",
        "/Users",
        "/etc",
        "/tmp",
        "(subpath \"/System\")",
        "/System/Volumes/Data",
        "(allow default",
        "(allow process-info-listpids",
    ] {
        assert!(!forms.iter().any(|f| f.contains(never)), "{never}");
    }
}
