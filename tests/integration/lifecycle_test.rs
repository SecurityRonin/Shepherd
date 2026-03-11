//! End-to-end integration test for the Shepherd lifecycle:
//! 1. Detect triggers on a new project
//! 2. Generate product names
//! 3. Run quality gates
//! 4. Verify trigger system dismissal

use shepherd_core::gates::{self, GateConfig};
use shepherd_core::triggers;
use shepherd_core::namegen::{NameCandidate, NameGenResult, NameValidation, ValidationStatus};
use shepherd_core::logogen::LogoStyle;
use shepherd_core::northstar::phases::PHASES;

#[test]
fn test_trigger_detection_on_new_project() {
    let tmp = tempfile::tempdir().unwrap();

    std::fs::write(
        tmp.path().join("package.json"),
        r#"{"name": "untitled", "version": "0.1.0"}"#,
    ).unwrap();

    let suggestions = triggers::check_triggers(tmp.path(), &[]);

    assert!(suggestions.len() >= 2, "Expected at least 2 trigger suggestions, got {}", suggestions.len());

    let tool_names: Vec<&str> = suggestions.iter().map(|s| s.tool.as_str()).collect();
    assert!(tool_names.contains(&"name_generator"), "Expected name_generator trigger");
    assert!(tool_names.contains(&"north_star"), "Expected north_star trigger");
}

#[test]
fn test_trigger_dismissed_not_reshown() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(
        tmp.path().join("package.json"),
        r#"{"name": "untitled"}"#,
    ).unwrap();

    let dismissed = vec!["namegen_untitled".to_string()];
    let suggestions = triggers::check_triggers(tmp.path(), &dismissed);

    let has_namegen = suggestions.iter().any(|s| s.id == "namegen_untitled");
    assert!(!has_namegen, "Dismissed trigger should not reappear");
}

#[test]
fn test_trigger_cleared_after_fix() {
    let tmp = tempfile::tempdir().unwrap();

    std::fs::write(
        tmp.path().join("package.json"),
        r#"{"name": "shepherd", "version": "1.0.0"}"#,
    ).unwrap();

    std::fs::create_dir_all(tmp.path().join("docs")).unwrap();

    std::fs::create_dir_all(tmp.path().join("public")).unwrap();
    std::fs::write(tmp.path().join("public/favicon.ico"), "icon").unwrap();

    let suggestions = triggers::check_triggers(tmp.path(), &[]);
    assert!(suggestions.is_empty(), "Well-configured project should have no triggers, got: {:?}",
        suggestions.iter().map(|s| &s.tool).collect::<Vec<_>>());
}

#[test]
fn test_name_validation_sorting() {
    let result = NameGenResult {
        candidates: vec![
            NameCandidate {
                name: "BadName".into(),
                tagline: None,
                reasoning: "test".into(),
                validation: NameValidation {
                    overall_status: ValidationStatus::Conflicted,
                    ..Default::default()
                },
            },
            NameCandidate {
                name: "GoodName".into(),
                tagline: None,
                reasoning: "test".into(),
                validation: NameValidation {
                    overall_status: ValidationStatus::AllClear,
                    ..Default::default()
                },
            },
            NameCandidate {
                name: "OkayName".into(),
                tagline: None,
                reasoning: "test".into(),
                validation: NameValidation {
                    overall_status: ValidationStatus::Partial,
                    ..Default::default()
                },
            },
        ],
    };

    let sorted = result.sorted();
    assert_eq!(sorted.candidates[0].name, "GoodName", "AllClear should be first");
    assert_eq!(sorted.candidates[1].name, "OkayName", "Partial should be second");
    assert_eq!(sorted.candidates[2].name, "BadName", "Conflicted should be last");
}

#[test]
fn test_gate_config_with_project_detection() {
    let tmp = tempfile::tempdir().unwrap();

    std::fs::write(tmp.path().join("Cargo.toml"), "[package]\nname = \"test\"").unwrap();

    let project_type = gates::builtin::detect_project_type(tmp.path());
    assert_eq!(project_type, gates::builtin::ProjectType::Rust);

    let config = GateConfig::default();
    assert!(config.lint);
    assert!(config.format_check);
    assert!(config.type_check);
    assert!(config.test);
}

#[test]
fn test_northstar_phases_complete() {
    assert_eq!(PHASES.len(), 13);

    for (i, phase) in PHASES.iter().enumerate() {
        assert_eq!(phase.id, (i + 1) as u8);
    }

    let total_docs: usize = PHASES.iter().map(|p| p.output_documents.len()).sum();
    assert!(total_docs >= 16, "Expected at least 16 documents, got {total_docs}");
}

#[test]
fn test_logo_style_prompt_coverage() {
    let styles = [LogoStyle::Minimal, LogoStyle::Geometric, LogoStyle::Mascot, LogoStyle::Abstract];
    let hints: Vec<&str> = styles.iter().map(|s| s.prompt_hint()).collect();

    for i in 0..hints.len() {
        for j in (i + 1)..hints.len() {
            assert_ne!(hints[i], hints[j], "Logo style hints should be unique");
        }
    }
}

#[test]
fn test_gate_all_passed_helper() {
    use shepherd_core::gates::{GateResult, GateType};

    let all_pass = vec![
        GateResult { gate_name: "lint".into(), passed: true, output: "".into(), duration_ms: 100, gate_type: GateType::Lint },
        GateResult { gate_name: "test".into(), passed: true, output: "".into(), duration_ms: 500, gate_type: GateType::Test },
    ];
    assert!(gates::all_gates_passed(&all_pass));

    let some_fail = vec![
        GateResult { gate_name: "lint".into(), passed: true, output: "".into(), duration_ms: 100, gate_type: GateType::Lint },
        GateResult { gate_name: "test".into(), passed: false, output: "1 failed".into(), duration_ms: 500, gate_type: GateType::Test },
    ];
    assert!(!gates::all_gates_passed(&some_fail));
}
