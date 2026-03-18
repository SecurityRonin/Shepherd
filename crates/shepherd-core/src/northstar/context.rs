use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

use crate::northstar::{NorthStarAnalysis, PhaseStatus};

/// Generate a YAML-formatted AI context string from a completed analysis.
pub fn generate_ai_context(analysis: &NorthStarAnalysis) -> String {
    let mut yaml = String::new();

    yaml.push_str("# North Star AI Context\n");
    yaml.push_str("# Auto-generated from PMF analysis pipeline\n\n");

    yaml.push_str(&format!("product_name: \"{}\"\n", analysis.product_name));
    yaml.push_str(&format!(
        "description: \"{}\"\n\n",
        analysis.product_description
    ));

    yaml.push_str("strategic_context:\n");
    for phase in &analysis.phases_completed {
        if phase.status != PhaseStatus::Completed {
            continue;
        }
        yaml.push_str(&format!("  - phase: \"{}\"\n", phase.phase_name));
        yaml.push_str("    documents:\n");
        for doc in &phase.documents {
            yaml.push_str(&format!("      - title: \"{}\"\n", doc.title));
            yaml.push_str(&format!("        filename: \"{}\"\n", doc.filename));
        }
    }

    // Extract kill-list items if phase 7 is completed
    yaml.push_str("\nkill_list:\n");
    let has_kill_list = analysis
        .phases_completed
        .iter()
        .any(|p| p.phase_id == 7 && p.status == PhaseStatus::Completed);
    if has_kill_list {
        yaml.push_str("  - see: kill-list.md\n");
    } else {
        yaml.push_str("  - pending\n");
    }

    // Metrics reference
    yaml.push_str("\nmetrics:\n");
    let has_metrics = analysis
        .phases_completed
        .iter()
        .any(|p| p.phase_id == 6 && p.status == PhaseStatus::Completed);
    if has_metrics {
        yaml.push_str("  - see: north-star-metric.md\n");
    } else {
        yaml.push_str("  - pending\n");
    }

    // Architecture reference
    yaml.push_str("\narchitecture:\n");
    let has_arch = analysis
        .phases_completed
        .iter()
        .any(|p| p.phase_id == 10 && p.status == PhaseStatus::Completed);
    if has_arch {
        yaml.push_str("  - see: technical-architecture.md\n");
        yaml.push_str("  - see: api-design.md\n");
    } else {
        yaml.push_str("  - pending\n");
    }

    yaml
}

/// Write all North Star analysis output to the filesystem.
///
/// Creates `docs/northstar/` under the given base directory, writes all
/// generated documents and the `ai-context.yml` file.
pub fn write_northstar_output(analysis: &NorthStarAnalysis, base_dir: &Path) -> Result<Vec<String>> {
    let northstar_dir = base_dir.join("docs").join("northstar");
    fs::create_dir_all(&northstar_dir)
        .with_context(|| format!("Failed to create {}", northstar_dir.display()))?;

    let mut written_files = Vec::new();

    // Write all generated documents
    for phase in &analysis.phases_completed {
        // tarpaulin-start-ignore
        if phase.status != PhaseStatus::Completed {
            continue;
        }
        // tarpaulin-stop-ignore
        for doc in &phase.documents {
            let doc_path = northstar_dir.join(&doc.filename);
            fs::write(&doc_path, &doc.content)
                .with_context(|| format!("Failed to write {}", doc_path.display()))?;
            written_files.push(doc_path.to_string_lossy().to_string());
        }
    }

    // Write ai-context.yml
    let ai_context = generate_ai_context(analysis);
    let context_path = northstar_dir.join("ai-context.yml");
    fs::write(&context_path, &ai_context)
        .with_context(|| format!("Failed to write {}", context_path.display()))?;
    written_files.push(context_path.to_string_lossy().to_string());

    Ok(written_files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::northstar::{GeneratedDocument, PhaseResult};

    fn sample_analysis() -> NorthStarAnalysis {
        NorthStarAnalysis {
            product_name: "TestApp".to_string(),
            product_description: "A test product for unit testing".to_string(),
            phases_completed: vec![
                PhaseResult {
                    phase_id: 1,
                    phase_name: "Product Vision".to_string(),
                    status: PhaseStatus::Completed,
                    output: "Vision output here".to_string(),
                    documents: vec![GeneratedDocument {
                        title: "Product Vision".to_string(),
                        filename: "product-vision.md".to_string(),
                        content: "# Product Vision\nVision content".to_string(),
                        doc_type: "markdown".to_string(),
                    }],
                },
                PhaseResult {
                    phase_id: 6,
                    phase_name: "North Star Metric".to_string(),
                    status: PhaseStatus::Completed,
                    output: "Metric output here".to_string(),
                    documents: vec![GeneratedDocument {
                        title: "North Star Metric".to_string(),
                        filename: "north-star-metric.md".to_string(),
                        content: "# North Star Metric\nMetric content".to_string(),
                        doc_type: "markdown".to_string(),
                    }],
                },
                PhaseResult {
                    phase_id: 7,
                    phase_name: "Feature Kill List".to_string(),
                    status: PhaseStatus::Completed,
                    output: "Kill list output".to_string(),
                    documents: vec![GeneratedDocument {
                        title: "Kill List".to_string(),
                        filename: "kill-list.md".to_string(),
                        content: "# Kill List\nKill list content".to_string(),
                        doc_type: "markdown".to_string(),
                    }],
                },
                PhaseResult {
                    phase_id: 10,
                    phase_name: "Technical Architecture".to_string(),
                    status: PhaseStatus::Completed,
                    output: "Architecture output".to_string(),
                    documents: vec![
                        GeneratedDocument {
                            title: "Technical Architecture".to_string(),
                            filename: "technical-architecture.md".to_string(),
                            content: "# Technical Architecture\nArch content".to_string(),
                            doc_type: "markdown".to_string(),
                        },
                        GeneratedDocument {
                            title: "API Design".to_string(),
                            filename: "api-design.md".to_string(),
                            content: "# API Design\nAPI content".to_string(),
                            doc_type: "markdown".to_string(),
                        },
                    ],
                },
            ],
            ai_context: None,
        }
    }

    #[test]
    fn generate_ai_context_basic() {
        let analysis = sample_analysis();
        let context = generate_ai_context(&analysis);

        assert!(context.contains("product_name: \"TestApp\""));
        assert!(context.contains("description: \"A test product for unit testing\""));
        assert!(context.contains("strategic_context:"));
        assert!(context.contains("Product Vision"));
        assert!(context.contains("product-vision.md"));
        assert!(context.contains("kill_list:"));
        assert!(context.contains("see: kill-list.md"));
        assert!(context.contains("metrics:"));
        assert!(context.contains("see: north-star-metric.md"));
        assert!(context.contains("architecture:"));
        assert!(context.contains("see: technical-architecture.md"));
        assert!(context.contains("see: api-design.md"));
    }

    #[test]
    fn generate_ai_context_skips_incomplete() {
        let analysis = NorthStarAnalysis {
            product_name: "FailApp".to_string(),
            product_description: "A failing product".to_string(),
            phases_completed: vec![
                PhaseResult {
                    phase_id: 1,
                    phase_name: "Product Vision".to_string(),
                    status: PhaseStatus::Completed,
                    output: "Vision output".to_string(),
                    documents: vec![GeneratedDocument {
                        title: "Product Vision".to_string(),
                        filename: "product-vision.md".to_string(),
                        content: "# Vision".to_string(),
                        doc_type: "markdown".to_string(),
                    }],
                },
                PhaseResult {
                    phase_id: 2,
                    phase_name: "Target Audience".to_string(),
                    status: PhaseStatus::Failed,
                    output: "Phase failed: API error".to_string(),
                    documents: vec![],
                },
                PhaseResult {
                    phase_id: 3,
                    phase_name: "Problem Statement".to_string(),
                    status: PhaseStatus::Skipped,
                    output: String::new(),
                    documents: vec![],
                },
            ],
            ai_context: None,
        };

        let context = generate_ai_context(&analysis);

        // Should include completed phase
        assert!(context.contains("Product Vision"));
        assert!(context.contains("product-vision.md"));

        // Should NOT include failed/skipped phases in strategic_context
        assert!(!context.contains("Target Audience"));
        assert!(!context.contains("Problem Statement"));

        // Kill list, metrics, architecture should show pending
        assert!(context.contains("kill_list:\n  - pending"));
        assert!(context.contains("metrics:\n  - pending"));
        assert!(context.contains("architecture:\n  - pending"));
    }

    #[test]
    fn write_northstar_output_skips_non_completed_phases() {
        let tmp = tempfile::tempdir().unwrap();
        let analysis = NorthStarAnalysis {
            product_name: "SkipApp".to_string(),
            product_description: "Test skipping".to_string(),
            phases_completed: vec![
                PhaseResult {
                    phase_id: 1,
                    phase_name: "Product Vision".to_string(),
                    status: PhaseStatus::Completed,
                    output: "Vision output".to_string(),
                    documents: vec![GeneratedDocument {
                        title: "Product Vision".to_string(),
                        filename: "product-vision.md".to_string(),
                        content: "# Vision".to_string(),
                        doc_type: "markdown".to_string(),
                    }],
                },
                PhaseResult {
                    phase_id: 2,
                    phase_name: "Target Audience".to_string(),
                    status: PhaseStatus::Failed,
                    output: "Failed".to_string(),
                    documents: vec![GeneratedDocument {
                        title: "Target Audience".to_string(),
                        filename: "target-audience.md".to_string(),
                        content: "# Audience".to_string(),
                        doc_type: "markdown".to_string(),
                    }],
                },
                PhaseResult {
                    phase_id: 3,
                    phase_name: "Problem Statement".to_string(),
                    status: PhaseStatus::Skipped,
                    output: "Skipped".to_string(),
                    documents: vec![GeneratedDocument {
                        title: "Problem Statement".to_string(),
                        filename: "problem-statement.md".to_string(),
                        content: "# Problem".to_string(),
                        doc_type: "markdown".to_string(),
                    }],
                },
            ],
            ai_context: None,
        };

        let files = write_northstar_output(&analysis, tmp.path()).unwrap();
        let northstar_dir = tmp.path().join("docs").join("northstar");

        // Only the completed phase doc + ai-context.yml should be written
        assert_eq!(files.len(), 2); // product-vision.md + ai-context.yml
        assert!(northstar_dir.join("product-vision.md").exists());
        assert!(!northstar_dir.join("target-audience.md").exists());
        assert!(!northstar_dir.join("problem-statement.md").exists());
        assert!(northstar_dir.join("ai-context.yml").exists());
    }

    #[test]
    fn write_northstar_output_creates_files() {
        let tmp = tempfile::tempdir().unwrap();
        let analysis = sample_analysis();

        let files = write_northstar_output(&analysis, tmp.path()).unwrap();

        // Should have written documents + ai-context.yml
        // 1 (vision) + 1 (metric) + 1 (kill-list) + 2 (arch + api) + 1 (ai-context) = 6
        assert_eq!(files.len(), 6);

        // Check that docs/northstar/ directory was created
        let northstar_dir = tmp.path().join("docs").join("northstar");
        assert!(northstar_dir.exists());

        // Check specific files exist
        assert!(northstar_dir.join("product-vision.md").exists());
        assert!(northstar_dir.join("north-star-metric.md").exists());
        assert!(northstar_dir.join("kill-list.md").exists());
        assert!(northstar_dir.join("technical-architecture.md").exists());
        assert!(northstar_dir.join("api-design.md").exists());
        assert!(northstar_dir.join("ai-context.yml").exists());

        // Check content of a document
        let vision_content =
            fs::read_to_string(northstar_dir.join("product-vision.md")).unwrap();
        assert!(vision_content.contains("# Product Vision"));

        // Check ai-context.yml content
        let context_content =
            fs::read_to_string(northstar_dir.join("ai-context.yml")).unwrap();
        assert!(context_content.contains("product_name: \"TestApp\""));
    }
}
