use commit_msg_lint::{config::Config, lint::lint};

#[test]
fn autosquash_header_ignores_the_entire_message_by_default() {
    let message = format!("fixup! not conventional {}\nbody\t\n", "x".repeat(80));
    assert!(lint(&message, &Config::default()).is_empty());
}

#[test]
fn autosquash_requires_a_literal_space_and_allows_any_target() {
    for header in ["fixup! ", "amend! ", "squash! ", "fixup! amend! invalid"] {
        assert!(lint(header, &Config::default()).is_empty(), "{header:?}");
    }
    for header in ["fixup!invalid", "Fixup! invalid", "squash!\tinvalid"] {
        assert!(!lint(header, &Config::default()).is_empty(), "{header:?}");
    }
}

#[test]
fn merge_and_revert_headers_ignore_the_whole_message() {
    for header in [
        "Merge branch x",
        "Merge tag x",
        "Merge remote-tracking branch x",
        "Merge pull request x",
        "Merge x into y",
        "Merged x in y",
        "Merged x into y",
        "Merged PR x: y",
        "Automatic merge",
        "Auto-merged x into y",
        "Revert x",
        "Reapply x",
        "revert x",
        "reapply x",
    ] {
        let input = format!("{header} {}\nbody\t", "x".repeat(80));
        assert!(lint(&input, &Config::default()).is_empty(), "{header:?}");
    }
}

#[test]
fn built_in_groups_can_be_disabled_independently() {
    for (group, header, other) in [
        ("merge", "Merge branch x", "fixup! x"),
        ("revert", "Revert x", "Merge branch x"),
        ("autosquash", "fixup! x", "Revert x"),
    ] {
        let config =
            Config::parse(&format!(r#"{{"version":1,"ignore":{{"{group}":false}}}}"#)).unwrap();
        assert!(!lint(header, &config).is_empty(), "{group}");
        assert!(lint(other, &config).is_empty(), "{group}");
    }
}

#[test]
fn custom_patterns_match_header_substrings_with_built_ins_disabled() {
    let config = Config::parse(r#"{"version":1,"ignore":{"merge":false,"revert":false,"autosquash":false,"header-patterns":["日本語", "^custom: "]}}"#).unwrap();
    assert!(lint("invalid 日本語 header\nbody\t", &config).is_empty());
    assert!(lint("custom: anything", &config).is_empty());
    assert!(!lint("invalid header\n\n日本語", &config).is_empty());
}

#[test]
fn invalid_custom_patterns_are_rejected_before_any_message_can_be_ignored() {
    for pattern in ["", "[", r"\p{Greek}", r"(?=x)"] {
        let json =
            serde_json::json!({"version": 1, "ignore": {"header-patterns": [".*", pattern]}});
        assert!(Config::parse(&json.to_string()).is_err(), "{pattern:?}");
        assert!(
            commit_msg_lint::config::HeaderPattern::new(pattern).is_err(),
            "{pattern:?}"
        );
    }
}

#[test]
fn custom_patterns_use_regex_lite_ascii_character_classes_and_anchors() {
    for (pattern, matching, nonmatching) in [
        (r"^\w+$", "bad_3", "bad_é"),
        (r"^x\sy$", "x y", "x\u{00a0}y"),
        (r"^x\dy$", "x3y", "x٣y"),
    ] {
        let json = serde_json::json!({"version": 1, "ignore": {"header-patterns": [pattern]}});
        let config = Config::parse(&json.to_string()).unwrap();
        assert!(lint(matching, &config).is_empty(), "{pattern}");
        assert!(!lint(nonmatching, &config).is_empty(), "{pattern}");
    }
    let config = Config::parse(r#"{"version":1,"ignore":{"header-patterns":[".*"]}}"#).unwrap();
    assert!(lint("", &config).is_empty());
}

#[test]
fn only_the_first_header_is_considered_and_nonmatching_messages_keep_severity() {
    let config = Config::parse(r#"{"version":1,"rules":{"header-format":{"severity":"warning"},"subject-full-stop":{"severity":"warning"}}}"#).unwrap();
    for input in [
        "invalid\n\nMerge branch x",
        "merge branch x",
        "Merge\tbranch x",
        "x Merge branch y",
        "Merge x Into y",
    ] {
        let diagnostics = lint(input, &config);
        assert!(
            diagnostics.iter().any(|d| d.rule == "header-format"
                && d.severity == commit_msg_lint::config::Severity::Warning),
            "{input:?}"
        );
    }
    assert!(
        lint("revert: keep this.", &config)
            .iter()
            .any(|d| d.rule == "subject-full-stop"
                && d.severity == commit_msg_lint::config::Severity::Warning)
    );
}

#[test]
fn filtered_lint_matches_the_first_retained_header_but_raw_lint_does_not_filter() {
    let config = Config::default();
    let message = "# comment\r\nfixup! invalid\r\nbody\t\r\n";
    assert!(!lint(message, &config).is_empty());
    assert!(commit_msg_lint::lint::lint_filtered(message, &config, "#", true, true).is_empty());
}

#[test]
fn callers_can_configure_ignore_groups_and_validated_patterns_directly() {
    let mut config = Config::default();
    config.ignore.autosquash = false;
    config
        .ignore
        .header_patterns
        .push(commit_msg_lint::config::HeaderPattern::new("^special$").unwrap());
    assert!(!lint("fixup! target", &config).is_empty());
    assert!(lint("special", &config).is_empty());
    assert_eq!(config.ignore.header_patterns[0].as_str(), "^special$");
}
