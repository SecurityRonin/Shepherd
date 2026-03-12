use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use shepherd_core::logogen::{self, ExportedFile, LogoGenInput, LogoStyle};
use std::path::PathBuf;
use std::sync::Arc;

use crate::state::AppState;

// ── Request / Response types ─────────────────────────────────────────

/// Request body for logo generation.
#[derive(Debug, Deserialize)]
pub struct LogoGenRequest {
    pub product_name: String,
    #[serde(default)]
    pub product_description: Option<String>,
    pub style: String,
    #[serde(default)]
    pub colors: Vec<String>,
}

/// Response body for logo generation.
#[derive(Debug, Serialize)]
pub struct LogoGenResponse {
    pub variants: Vec<VariantResponse>,
}

/// A single variant in the response.
#[derive(Debug, Serialize)]
pub struct VariantResponse {
    pub index: u8,
    pub image_data: String,
    pub is_url: bool,
}

/// Request body for icon export.
#[derive(Debug, Deserialize)]
pub struct ExportRequest {
    pub image_base64: String,
    pub product_name: String,
    #[serde(default)]
    pub output_dir: Option<String>,
}

/// Response body for icon export.
#[derive(Debug, Serialize)]
pub struct ExportResponse {
    pub files: Vec<ExportedFileResponse>,
}

/// A single exported file in the response.
#[derive(Debug, Serialize)]
pub struct ExportedFileResponse {
    pub path: String,
    pub format: String,
    pub size_bytes: u64,
    pub dimensions: Option<(u32, u32)>,
}

impl From<ExportedFile> for ExportedFileResponse {
    fn from(f: ExportedFile) -> Self {
        Self {
            path: f.path,
            format: f.format,
            size_bytes: f.size_bytes,
            dimensions: f.dimensions,
        }
    }
}

// ── Helpers ──────────────────────────────────────────────────────────

/// Map a style string to a `LogoStyle`, defaulting to `Minimal`.
fn parse_style(s: &str) -> LogoStyle {
    match s.to_lowercase().as_str() {
        "minimal" => LogoStyle::Minimal,
        "geometric" => LogoStyle::Geometric,
        "mascot" => LogoStyle::Mascot,
        "abstract" => LogoStyle::Abstract,
        _ => LogoStyle::Minimal,
    }
}

// ── Handlers ─────────────────────────────────────────────────────────

/// POST /api/logogen — generate logo variants.
#[tracing::instrument(skip(state, req))]
pub async fn generate_logo(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LogoGenRequest>,
) -> Result<Json<LogoGenResponse>, (StatusCode, Json<serde_json::Value>)> {
    let provider = state.llm_provider.as_ref().ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "LLM provider not configured"
            })),
        )
    })?;

    let input = LogoGenInput {
        product_name: req.product_name,
        product_description: req.product_description,
        style: parse_style(&req.style),
        colors: req.colors,
        variants: 4,
    };

    let result = logogen::generate::generate_logos(provider.as_ref(), &input)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": format!("Logo generation failed: {e}")
                })),
            )
        })?;

    let variants = result
        .variants
        .into_iter()
        .map(|v| VariantResponse {
            index: v.index,
            image_data: v.png_data,
            is_url: false,
        })
        .collect();

    Ok(Json(LogoGenResponse { variants }))
}

/// POST /api/logogen/export — export icons from a selected variant.
#[tracing::instrument(skip(req))]
pub async fn export_icons(
    Json(req): Json<ExportRequest>,
) -> Result<Json<ExportResponse>, (StatusCode, Json<serde_json::Value>)> {
    let output_dir = req
        .output_dir
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("icons"));

    let export = logogen::export::export_icons(&req.image_base64, &output_dir, &req.product_name)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": format!("Icon export failed: {e}")
                })),
            )
        })?;

    let files = export.files.into_iter().map(ExportedFileResponse::from).collect();

    Ok(Json(ExportResponse { files }))
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_style_known() {
        assert_eq!(parse_style("minimal"), LogoStyle::Minimal);
        assert_eq!(parse_style("geometric"), LogoStyle::Geometric);
        assert_eq!(parse_style("mascot"), LogoStyle::Mascot);
        assert_eq!(parse_style("abstract"), LogoStyle::Abstract);
    }

    #[test]
    fn parse_style_case_insensitive() {
        assert_eq!(parse_style("MINIMAL"), LogoStyle::Minimal);
        assert_eq!(parse_style("Geometric"), LogoStyle::Geometric);
        assert_eq!(parse_style("MASCOT"), LogoStyle::Mascot);
        assert_eq!(parse_style("Abstract"), LogoStyle::Abstract);
    }

    #[test]
    fn parse_style_unknown_defaults_to_minimal() {
        assert_eq!(parse_style("unknown"), LogoStyle::Minimal);
        assert_eq!(parse_style(""), LogoStyle::Minimal);
        assert_eq!(parse_style("fancy"), LogoStyle::Minimal);
    }

    #[test]
    fn exported_file_response_from() {
        let file = ExportedFile {
            path: "/tmp/icon-64.png".to_string(),
            size_bytes: 1234,
            format: "png".to_string(),
            dimensions: Some((64, 64)),
        };

        let response: ExportedFileResponse = file.into();
        assert_eq!(response.path, "/tmp/icon-64.png");
        assert_eq!(response.size_bytes, 1234);
        assert_eq!(response.format, "png");
        assert_eq!(response.dimensions, Some((64, 64)));
    }

    #[test]
    fn exported_file_response_no_dimensions() {
        let file = ExportedFile {
            path: "/tmp/favicon.ico".to_string(),
            size_bytes: 567,
            format: "ico".to_string(),
            dimensions: None,
        };

        let response: ExportedFileResponse = file.into();
        assert!(response.dimensions.is_none());
    }

    #[test]
    fn logogen_request_deserialize() {
        let json = r##"{
            "product_name": "Acme",
            "product_description": "A cool product",
            "style": "geometric",
            "colors": ["#FF0000", "#00FF00"]
        }"##;

        let req: LogoGenRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.product_name, "Acme");
        assert_eq!(req.product_description, Some("A cool product".to_string()));
        assert_eq!(req.style, "geometric");
        assert_eq!(req.colors, vec!["#FF0000", "#00FF00"]);
    }

    #[test]
    fn logogen_request_minimal() {
        let json = r#"{"product_name": "Foo", "style": "minimal"}"#;
        let req: LogoGenRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.product_name, "Foo");
        assert!(req.product_description.is_none());
        assert!(req.colors.is_empty());
    }

    #[test]
    fn export_request_deserialize() {
        let json = r#"{
            "image_base64": "abc123",
            "product_name": "TestApp",
            "output_dir": "/tmp/icons"
        }"#;

        let req: ExportRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.image_base64, "abc123");
        assert_eq!(req.product_name, "TestApp");
        assert_eq!(req.output_dir, Some("/tmp/icons".to_string()));
    }

    #[test]
    fn export_request_defaults() {
        let json = r#"{"image_base64": "data", "product_name": "App"}"#;
        let req: ExportRequest = serde_json::from_str(json).unwrap();
        assert!(req.output_dir.is_none());
    }

    #[test]
    fn logogen_response_serialize() {
        let response = LogoGenResponse {
            variants: vec![
                VariantResponse {
                    index: 0,
                    image_data: "base64data".to_string(),
                    is_url: false,
                },
                VariantResponse {
                    index: 1,
                    image_data: "https://example.com/img.png".to_string(),
                    is_url: true,
                },
            ],
        };

        let json = serde_json::to_value(&response).unwrap();
        let variants = json["variants"].as_array().unwrap();
        assert_eq!(variants.len(), 2);
        assert_eq!(variants[0]["index"], 0);
        assert!(!variants[0]["is_url"].as_bool().unwrap());
        assert!(variants[1]["is_url"].as_bool().unwrap());
    }

    #[test]
    fn export_response_serialize() {
        let response = ExportResponse {
            files: vec![ExportedFileResponse {
                path: "/tmp/icon-512.png".to_string(),
                format: "png".to_string(),
                size_bytes: 9999,
                dimensions: Some((512, 512)),
            }],
        };

        let json = serde_json::to_value(&response).unwrap();
        let files = json["files"].as_array().unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0]["size_bytes"], 9999);
    }
}
