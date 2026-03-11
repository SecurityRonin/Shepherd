pub mod export;
pub mod generate;

use serde::{Deserialize, Serialize};

/// Logo style for generation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LogoStyle {
    Minimal,
    Geometric,
    Mascot,
    Abstract,
}

impl LogoStyle {
    /// Return a descriptive prompt hint for this style.
    pub fn prompt_hint(&self) -> &str {
        match self {
            LogoStyle::Minimal => "clean minimal design with simple shapes and limited colors",
            LogoStyle::Geometric => "geometric patterns with precise shapes and symmetry",
            LogoStyle::Mascot => "friendly character mascot with personality and appeal",
            LogoStyle::Abstract => "abstract artistic design with creative fluid forms",
        }
    }
}

/// Input parameters for logo generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogoGenInput {
    pub product_name: String,
    #[serde(default)]
    pub product_description: Option<String>,
    pub style: LogoStyle,
    #[serde(default)]
    pub colors: Vec<String>,
    #[serde(default = "default_variants")]
    pub variants: u8,
}

fn default_variants() -> u8 {
    4
}

impl Default for LogoGenInput {
    fn default() -> Self {
        Self {
            product_name: String::new(),
            product_description: None,
            style: LogoStyle::Minimal,
            colors: Vec::new(),
            variants: 4,
        }
    }
}

/// A single logo variant from generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogoVariant {
    pub index: u8,
    pub png_data: String,
    pub selected: bool,
}

/// Result of logo generation containing multiple variants.
#[derive(Debug, Clone)]
pub struct LogoGenResult {
    pub variants: Vec<LogoVariant>,
    pub style: LogoStyle,
}

/// Result of icon export containing all exported files.
#[derive(Debug, Clone)]
pub struct IconExport {
    pub files: Vec<ExportedFile>,
}

/// A single exported file with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportedFile {
    pub path: String,
    pub size_bytes: u64,
    pub format: String,
    pub dimensions: Option<(u32, u32)>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logo_style_prompt_hints() {
        assert!(LogoStyle::Minimal.prompt_hint().contains("minimal"));
        assert!(LogoStyle::Geometric.prompt_hint().contains("geometric"));
        assert!(LogoStyle::Mascot.prompt_hint().contains("mascot"));
        assert!(LogoStyle::Abstract.prompt_hint().contains("abstract"));

        // Each hint should be non-empty and descriptive
        for style in [
            LogoStyle::Minimal,
            LogoStyle::Geometric,
            LogoStyle::Mascot,
            LogoStyle::Abstract,
        ] {
            let hint = style.prompt_hint();
            assert!(!hint.is_empty());
            assert!(hint.len() > 10, "Hint should be descriptive: {hint}");
        }
    }

    #[test]
    fn default_variants() {
        let input = LogoGenInput::default();
        assert_eq!(input.variants, 4);
        assert!(input.colors.is_empty());
        assert!(input.product_description.is_none());
        assert_eq!(input.style, LogoStyle::Minimal);
    }

    #[test]
    fn logo_gen_input_serde() {
        let input = LogoGenInput {
            product_name: "TestApp".to_string(),
            product_description: Some("A test application".to_string()),
            style: LogoStyle::Geometric,
            colors: vec!["#ff0000".to_string(), "#00ff00".to_string()],
            variants: 3,
        };

        let json = serde_json::to_string(&input).unwrap();
        let deserialized: LogoGenInput = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.product_name, "TestApp");
        assert_eq!(
            deserialized.product_description,
            Some("A test application".to_string())
        );
        assert_eq!(deserialized.style, LogoStyle::Geometric);
        assert_eq!(deserialized.colors.len(), 2);
        assert_eq!(deserialized.variants, 3);

        // Verify snake_case serialization of style
        assert!(json.contains("\"geometric\""));

        // Verify deserialization with defaults (missing optional fields)
        let minimal_json = r#"{"product_name":"Foo","style":"mascot"}"#;
        let minimal: LogoGenInput = serde_json::from_str(minimal_json).unwrap();
        assert_eq!(minimal.product_name, "Foo");
        assert_eq!(minimal.style, LogoStyle::Mascot);
        assert!(minimal.colors.is_empty());
        assert_eq!(minimal.variants, 4);
        assert!(minimal.product_description.is_none());
    }
}
