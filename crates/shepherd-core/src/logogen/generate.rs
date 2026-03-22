use anyhow::Result;

use crate::llm::{ImageGenRequest, ImageGenResponse, LlmProvider};
use crate::logogen::{LogoGenInput, LogoGenResult, LogoVariant};

/// Build a logo generation prompt from the input parameters.
pub fn build_logo_prompt(input: &LogoGenInput) -> String {
    let mut parts = Vec::new();

    parts.push(format!(
        "Design a professional logo for a product called \"{}\".",
        input.product_name
    ));

    if let Some(ref desc) = input.product_description {
        parts.push(format!("Product description: {desc}."));
    }

    parts.push(format!("Style: {}.", input.style.prompt_hint()));

    if !input.colors.is_empty() {
        parts.push(format!("Color palette: {}.", input.colors.join(", ")));
    }

    parts.push(
        "The logo should be suitable for app icons, favicons, and marketing materials. \
         Clean background, scalable vector-style design, high contrast."
            .to_string(),
    );

    parts.join(" ")
}

/// Generate logo variants using an LLM image generation provider.
pub async fn generate_logos(llm: &dyn LlmProvider, input: &LogoGenInput) -> Result<LogoGenResult> {
    let prompt = build_logo_prompt(input);

    let mut request = ImageGenRequest::new(prompt);
    request.n = input.variants as u32;

    let response: ImageGenResponse = llm.generate_image(&request).await?;

    let variants = response
        .images
        .into_iter()
        .enumerate()
        .map(|(i, img)| LogoVariant {
            index: i as u8,
            png_data: img.data,
            selected: i == 0,
        })
        .collect();

    Ok(LogoGenResult {
        variants,
        style: input.style.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logogen::LogoStyle;

    #[test]
    fn build_logo_prompt_minimal() {
        let input = LogoGenInput {
            product_name: "Acme".to_string(),
            product_description: Some("A tool for building things".to_string()),
            style: LogoStyle::Minimal,
            colors: vec!["blue".to_string(), "white".to_string()],
            variants: 4,
        };

        let prompt = build_logo_prompt(&input);

        assert!(prompt.contains("Acme"), "Should contain product name");
        assert!(
            prompt.contains("A tool for building things"),
            "Should contain description"
        );
        assert!(
            prompt.contains("minimal"),
            "Should contain style hint keyword"
        );
        assert!(
            prompt.contains("blue, white"),
            "Should contain color palette"
        );
        assert!(
            prompt.contains("professional"),
            "Should contain professional instruction"
        );
        assert!(
            prompt.contains("scalable"),
            "Should contain scalable instruction"
        );
    }

    #[test]
    fn build_logo_prompt_no_description_no_colors() {
        let input = LogoGenInput {
            product_name: "Zeta".to_string(),
            product_description: None,
            style: LogoStyle::Abstract,
            colors: vec![],
            variants: 2,
        };

        let prompt = build_logo_prompt(&input);

        assert!(prompt.contains("Zeta"), "Should contain product name");
        assert!(
            !prompt.contains("Product description:"),
            "Should not contain description section"
        );
        assert!(
            prompt.contains("abstract"),
            "Should contain style hint keyword"
        );
        assert!(
            !prompt.contains("Color palette:"),
            "Should not contain color palette section"
        );
        assert!(
            prompt.contains("professional"),
            "Should contain professional instruction"
        );
    }

    #[test]
    fn build_logo_prompt_geometric_style() {
        let input = LogoGenInput {
            product_name: "Delta".to_string(),
            product_description: None,
            style: LogoStyle::Geometric,
            colors: vec!["red".to_string()],
            variants: 1,
        };
        let prompt = build_logo_prompt(&input);
        assert!(prompt.contains("Delta"));
        assert!(prompt.contains("geometric"));
        assert!(prompt.contains("red"));
        assert!(!prompt.contains("description"));
    }

    #[test]
    fn build_logo_prompt_all_styles() {
        for style in [
            LogoStyle::Minimal,
            LogoStyle::Abstract,
            LogoStyle::Geometric,
            LogoStyle::Mascot,
        ] {
            let input = LogoGenInput {
                product_name: "Test".to_string(),
                product_description: None,
                style: style.clone(),
                colors: vec![],
                variants: 1,
            };
            let prompt = build_logo_prompt(&input);
            assert!(
                prompt.contains(&style.prompt_hint().to_lowercase())
                    || prompt.contains("professional")
            );
        }
    }

    #[tokio::test]
    async fn generate_logos_with_mock() {
        use crate::llm::{
            GeneratedImage, ImageGenRequest, ImageGenResponse, LlmProvider, LlmRequest, LlmResponse,
        };

        struct MockImageProvider;

        #[async_trait::async_trait]
        impl LlmProvider for MockImageProvider {
            async fn chat(&self, _request: &LlmRequest) -> anyhow::Result<LlmResponse> {
                anyhow::bail!("chat not supported by image-only mock")
            }
            async fn generate_image(
                &self,
                request: &ImageGenRequest,
            ) -> anyhow::Result<ImageGenResponse> {
                let images: Vec<GeneratedImage> = (0..request.n)
                    .map(|i| GeneratedImage {
                        data: format!("base64_png_data_{}", i),
                        is_url: false,
                    })
                    .collect();
                Ok(ImageGenResponse { images })
            }
            fn name(&self) -> &str {
                "mock-image"
            }
        }

        let input = LogoGenInput {
            product_name: "TestApp".to_string(),
            product_description: Some("A test application".to_string()),
            style: LogoStyle::Minimal,
            colors: vec!["blue".to_string()],
            variants: 3,
        };

        let result = generate_logos(&MockImageProvider, &input).await.unwrap();
        assert_eq!(result.variants.len(), 3);
        assert_eq!(result.style, LogoStyle::Minimal);
        assert_eq!(result.variants[0].index, 0);
        assert!(result.variants[0].selected);
        assert!(!result.variants[1].selected);
        assert!(!result.variants[2].selected);
        assert_eq!(result.variants[0].png_data, "base64_png_data_0");
        assert_eq!(result.variants[2].png_data, "base64_png_data_2");
    }

    #[tokio::test]
    async fn generate_logos_error_propagates() {
        use crate::llm::{ImageGenRequest, ImageGenResponse, LlmProvider, LlmRequest, LlmResponse};

        struct FailImageProvider;

        #[async_trait::async_trait]
        impl LlmProvider for FailImageProvider {
            async fn chat(&self, _request: &LlmRequest) -> anyhow::Result<LlmResponse> {
                anyhow::bail!("chat not supported by image-only mock")
            }
            async fn generate_image(
                &self,
                _request: &ImageGenRequest,
            ) -> anyhow::Result<ImageGenResponse> {
                anyhow::bail!("Image generation quota exceeded")
            }
            fn name(&self) -> &str {
                "fail-image"
            }
        }

        let input = LogoGenInput {
            product_name: "Test".to_string(),
            product_description: None,
            style: LogoStyle::Abstract,
            colors: vec![],
            variants: 1,
        };

        let result = generate_logos(&FailImageProvider, &input).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("quota exceeded"));
    }
}
