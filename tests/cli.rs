use std::{
    io::Write,
    process::{Command, Stdio},
};

#[test]
fn json_session_preserves_state_and_returns_failure_on_errors() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_rustj"))
        .arg("--json")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"a =: i. 3\na + 2\n1 2 + 3 4 5\na\n")
        .unwrap();
    let result = child.wait_with_output().unwrap();
    assert_eq!(result.status.code(), Some(1));
    let text = String::from_utf8(result.stdout).unwrap();
    let lines: Vec<_> = text.lines().collect();
    assert_eq!(lines.len(), 4);
    assert_eq!(lines[0], "{\"silent\":true}");
    assert_eq!(lines[1], "{\"type\":4,\"shape\":[3],\"data\":[2,3,4]}");
    assert_eq!(lines[2], "{\"error\":\"length error\"}");
    assert_eq!(lines[3], "{\"type\":4,\"shape\":[3],\"data\":[0,1,2]}");
}

#[test]
fn script_and_argument_failures_have_nonzero_status() {
    assert!(
        !Command::new(env!("CARGO_BIN_EXE_rustj"))
            .args(["-e"])
            .output()
            .unwrap()
            .status
            .success()
    );
    assert!(
        !Command::new(env!("CARGO_BIN_EXE_rustj"))
            .arg("nonexistent-script.ijs")
            .output()
            .unwrap()
            .status
            .success()
    );
    let result = Command::new(env!("CARGO_BIN_EXE_rustj"))
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/examples/milestone.ijs"
        ))
        .output()
        .unwrap();
    assert!(result.status.success());
    assert!(
        String::from_utf8(result.stdout)
            .unwrap()
            .ends_with("RustJ\n")
    );
}

#[test]
fn unsupported_definitions_never_execute_following_body_lines() {
    for source in [
        "f=:{{\ngoto_done.\nleaked=:99\nlabel_done.\n}}\nleaked\n",
        "f=:3 : 0\ngoto_done.\nleaked=:99\nlabel_done.\n)\nleaked\n",
        "f=:{{ 'unfinished\nleaked=:99\n}}\nleaked\n",
        "f=:verb define\nleaked=:99\n)\nleaked\n",
    ] {
        for semantic in [false, true] {
            let mut command = Command::new(env!("CARGO_BIN_EXE_rustj"));
            command.arg("--json");
            if semantic {
                command.arg("--semantic-reference");
            }
            let mut child = command
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap();
            child
                .stdin
                .take()
                .unwrap()
                .write_all(source.as_bytes())
                .unwrap();
            let result = child.wait_with_output().unwrap();
            assert_eq!(result.status.code(), Some(1));
            let text = String::from_utf8(result.stdout).unwrap();
            assert_eq!(text.lines().count(), 1, "{source}: {text}");
            assert!(!text.contains("99"));
            assert!(
                String::from_utf8(result.stderr)
                    .unwrap()
                    .contains("stopping input")
            );
        }
    }
}

#[test]
fn delimiter_text_in_a_failed_sentence_does_not_abort_later_sentences() {
    for source in ["'{{ }} : define'+1\n42\n", "1 2+1 2 3 NB. {{\n42\n"] {
        let mut child = Command::new(env!("CARGO_BIN_EXE_rustj"))
            .arg("--json")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(source.as_bytes())
            .unwrap();
        let result = child.wait_with_output().unwrap();
        assert_eq!(result.status.code(), Some(1));
        let text = String::from_utf8(result.stdout).unwrap();
        assert_eq!(text.lines().count(), 2, "{text}");
        assert!(text.ends_with("{\"type\":4,\"shape\":[],\"data\":[42]}\n"));
        assert!(
            !String::from_utf8(result.stderr)
                .unwrap()
                .contains("stopping input")
        );
    }
}

#[test]
fn human_errors_use_python_style_source_diagnostics() {
    let result = Command::new(env!("CARGO_BIN_EXE_rustj"))
        .args(["-e", "1 + )"])
        .output()
        .unwrap();
    assert!(!result.status.success());
    let stderr = String::from_utf8(result.stderr).unwrap();
    assert!(
        stderr.contains("File \"<command-line>\", line 1, column 5"),
        "{stderr}"
    );
    assert!(stderr.contains("1 + )"), "{stderr}");
    assert!(stderr.contains("^"), "{stderr}");
    assert!(stderr.contains("SyntaxError: unexpected )"), "{stderr}");
}

#[test]
fn human_length_errors_include_semantic_execution_context() {
    let result = Command::new(env!("CARGO_BIN_EXE_rustj"))
        .args(["-e", "2 3 + 4 5 6"])
        .output()
        .unwrap();
    assert!(!result.status.success());
    let stderr = String::from_utf8(result.stderr).unwrap();
    assert!(stderr.contains("LengthError"), "{stderr}");
    assert!(stderr.contains("while executing dyad +"), "{stderr}");
    assert!(stderr.contains("x: integer, rank 1, shape 2"), "{stderr}");
    assert!(stderr.contains("y: integer, rank 1, shape 3"), "{stderr}");
    assert!(stderr.contains("shapes 2 and 3 do not conform"), "{stderr}");
}

#[test]
fn definition_input_waits_for_closing_line_before_binding_code() {
    use std::{io::BufRead, sync::mpsc, time::Duration};
    for (header, closing) in [("f=:{{", "}}"), ("f=:3 : 0", ")")] {
        let mut child = Command::new(env!("CARGO_BIN_EXE_rustj"))
            .arg("--json")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut input = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let (sender, receiver) = mpsc::channel();
        let reader = std::thread::spawn(move || {
            let mut reader = std::io::BufReader::new(stdout);
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            sender.send(line).unwrap();
            let mut remaining = String::new();
            std::io::Read::read_to_string(&mut reader, &mut remaining).unwrap();
            remaining
        });
        writeln!(input, "{header}\nleaked=:99").unwrap();
        input.flush().unwrap();
        assert!(receiver.recv_timeout(Duration::from_millis(100)).is_err());
        writeln!(input, "{closing}\nleaked 0").unwrap();
        drop(input);
        let response = receiver.recv_timeout(Duration::from_secs(10)).unwrap();
        assert!(response.contains("silent"));
        let remaining = reader.join().unwrap();
        assert!(remaining.contains("value error"));
        assert!(!remaining.contains("99"));
        assert_eq!(child.wait().unwrap().code(), Some(1));
    }
}

#[test]
fn completed_definition_error_does_not_truncate_a_json_session() {
    for semantic in [false, true] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_rustj"));
        command.arg("--json");
        if semantic {
            command.arg("--semantic-reference");
        }
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(b"f=:+\nf=:3 : 'goto_done.' 1 2+1 2 3\nf 7\n")
            .unwrap();
        let result = child.wait_with_output().unwrap();
        assert_eq!(result.status.code(), Some(1));
        let text = String::from_utf8(result.stdout).unwrap();
        assert_eq!(text.lines().count(), 3, "{text}");
        assert!(text.contains("length error"));
        assert!(text.ends_with("{\"type\":4,\"shape\":[],\"data\":[7]}\n"));
    }
}
