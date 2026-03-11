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
pub async fn generate_logos(
    llm: &dyn LlmProvider,
    input: &LogoGenInput,
) -> Result<LogoGenResult> {
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
}
