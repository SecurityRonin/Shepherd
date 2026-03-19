//! Verify that required governance and documentation files exist.

use std::path::Path;

fn project_root() -> &'static Path {
    // CARGO_MANIFEST_DIR points to crates/shepherd-core; go up two levels for workspace root.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
}

#[test]
fn contributing_md_exists_and_has_required_sections() {
    let path = project_root().join("CONTRIBUTING.md");
    assert!(path.exists(), "CONTRIBUTING.md must exist");
    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("# Contributing"), "Must have title");
    assert!(
        content.contains("## Getting Started"),
        "Must have Getting Started section"
    );
    assert!(
        content.contains("## Pull Requests"),
        "Must have Pull Requests section"
    );
    assert!(content.contains("cargo test"), "Must mention running tests");
    assert!(content.contains("cargo fmt"), "Must mention formatting");
}

#[test]
fn security_md_exists_and_has_required_sections() {
    let path = project_root().join("SECURITY.md");
    assert!(path.exists(), "SECURITY.md must exist");
    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("# Security Policy"), "Must have title");
    assert!(content.contains("Reporting"), "Must have reporting section");
}

#[test]
fn code_of_conduct_exists() {
    let path = project_root().join("CODE_OF_CONDUCT.md");
    assert!(path.exists(), "CODE_OF_CONDUCT.md must exist");
    let content = std::fs::read_to_string(&path).unwrap();
    assert!(
        content.contains("Contributor Covenant") || content.contains("Code of Conduct"),
        "Must be Contributor Covenant or similar"
    );
}

#[test]
fn funding_yml_exists() {
    let path = project_root().join(".github/FUNDING.yml");
    assert!(path.exists(), ".github/FUNDING.yml must exist");
    let content = std::fs::read_to_string(&path).unwrap();
    assert!(
        content.contains("github:"),
        "Must have github sponsors entry"
    );
}

#[test]
fn github_issue_templates_exist() {
    let root = project_root();
    let bug = root.join(".github/ISSUE_TEMPLATE/bug_report.md");
    let feature = root.join(".github/ISSUE_TEMPLATE/feature_request.md");
    assert!(bug.exists(), "Bug report template must exist");
    assert!(feature.exists(), "Feature request template must exist");

    let bug_content = std::fs::read_to_string(&bug).unwrap();
    assert!(
        bug_content.contains("name:"),
        "Bug template must have YAML front matter"
    );

    let feature_content = std::fs::read_to_string(&feature).unwrap();
    assert!(
        feature_content.contains("name:"),
        "Feature template must have YAML front matter"
    );
}

#[test]
fn pr_template_exists() {
    let root = project_root();
    let pr = root.join(".github/PULL_REQUEST_TEMPLATE.md");
    assert!(pr.exists(), "PR template must exist");
    let content = std::fs::read_to_string(&pr).unwrap();
    assert!(
        content.contains("## Summary"),
        "PR template must have Summary section"
    );
}
