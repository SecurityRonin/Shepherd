pub mod context;
pub mod phases;

use serde::{Deserialize, Serialize};

/// Full analysis result for a product's North Star PMF journey.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NorthStarAnalysis {
    pub product_name: String,
    pub product_description: String,
    pub phases_completed: Vec<PhaseResult>,
    #[serde(default)]
    pub ai_context: Option<String>,
}

/// Result of executing a single phase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseResult {
    pub phase_id: u8,
    pub phase_name: String,
    pub status: PhaseStatus,
    pub output: String,
    #[serde(default)]
    pub documents: Vec<GeneratedDocument>,
}

/// Status of a phase in the analysis pipeline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PhaseStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
}

/// A document generated during a phase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedDocument {
    pub title: String,
    pub filename: String,
    pub content: String,
    pub doc_type: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phase_status_serde() {
        // Serialize
        let pending = serde_json::to_string(&PhaseStatus::Pending).unwrap();
        assert_eq!(pending, "\"pending\"");

        let running = serde_json::to_string(&PhaseStatus::Running).unwrap();
        assert_eq!(running, "\"running\"");

        let completed = serde_json::to_string(&PhaseStatus::Completed).unwrap();
        assert_eq!(completed, "\"completed\"");

        let failed = serde_json::to_string(&PhaseStatus::Failed).unwrap();
        assert_eq!(failed, "\"failed\"");

        let skipped = serde_json::to_string(&PhaseStatus::Skipped).unwrap();
        assert_eq!(skipped, "\"skipped\"");

        // Deserialize
        let from_str: PhaseStatus = serde_json::from_str("\"completed\"").unwrap();
        assert_eq!(from_str, PhaseStatus::Completed);

        let from_str2: PhaseStatus = serde_json::from_str("\"running\"").unwrap();
        assert_eq!(from_str2, PhaseStatus::Running);
    }

    #[test]
    fn north_star_analysis_roundtrip() {
        let analysis = NorthStarAnalysis {
            product_name: "TestApp".to_string(),
            product_description: "A test product".to_string(),
            phases_completed: vec![PhaseResult {
                phase_id: 1,
                phase_name: "Product Vision".to_string(),
                status: PhaseStatus::Completed,
                output: "Vision output".to_string(),
                documents: vec![GeneratedDocument {
                    title: "Vision Doc".to_string(),
                    filename: "vision.md".to_string(),
                    content: "# Vision".to_string(),
                    doc_type: "markdown".to_string(),
                }],
            }],
            ai_context: Some("context data".to_string()),
        };

        let json = serde_json::to_string(&analysis).unwrap();
        let deserialized: NorthStarAnalysis = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.product_name, "TestApp");
        assert_eq!(deserialized.phases_completed.len(), 1);
        assert_eq!(deserialized.phases_completed[0].phase_id, 1);
        assert_eq!(
            deserialized.phases_completed[0].status,
            PhaseStatus::Completed
        );
        assert_eq!(deserialized.phases_completed[0].documents.len(), 1);
        assert!(deserialized.ai_context.is_some());
    }

    #[test]
    fn generated_document_serde() {
        let doc = GeneratedDocument {
            title: "Test".to_string(),
            filename: "test.md".to_string(),
            content: "# Test Content".to_string(),
            doc_type: "markdown".to_string(),
        };

        let json = serde_json::to_string(&doc).unwrap();
        assert!(json.contains("test.md"));

        let deserialized: GeneratedDocument = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.title, "Test");
        assert_eq!(deserialized.doc_type, "markdown");
    }
}
