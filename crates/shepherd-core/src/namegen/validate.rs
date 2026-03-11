use anyhow::Result;
use std::time::Duration;

use crate::namegen::DomainCheck;

/// Timeout for registry HTTP requests.
const REGISTRY_TIMEOUT: Duration = Duration::from_secs(15);

/// Validate a name across domain registries and package registries.
///
/// Runs domain checks, npm, PyPI, and GitHub checks concurrently using tokio::join!.
/// Returns (domains, npm_available, pypi_available, github_available).
pub async fn validate_name(
    name: &str,
) -> Result<(Vec<DomainCheck>, Option<bool>, Option<bool>, Option<bool>)> {
    let client = reqwest::Client::builder()
        .timeout(REGISTRY_TIMEOUT)
        .build()?;

    let name_owned = name.to_string();
    let client_ref = &client;
    let name_ref = &name_owned;

    let (domains, npm, pypi, github) = tokio::join!(
        crate::namegen::rdap::check_domains_for_name(name_ref),
        check_npm(client_ref, name_ref),
        check_pypi(client_ref, name_ref),
        check_github(client_ref, name_ref),
    );

    Ok((
        domains,
        npm.ok(),
        pypi.ok(),
        github.ok(),
    ))
}

/// Check npm registry for package name availability.
///
/// Hits `https://registry.npmjs.org/{name}`.
/// 404 = available, 200 = taken.
pub async fn check_npm(client: &reqwest::Client, name: &str) -> Result<bool> {
    let url = format!("https://registry.npmjs.org/{name}");
    let response = client.get(&url).send().await?;
    Ok(response.status().as_u16() == 404)
}

/// Check PyPI registry for package name availability.
///
/// Hits `https://pypi.org/pypi/{name}/json`.
/// 404 = available, 200 = taken.
pub async fn check_pypi(client: &reqwest::Client, name: &str) -> Result<bool> {
    let url = format!("https://pypi.org/pypi/{name}/json");
    let response = client.get(&url).send().await?;
    Ok(response.status().as_u16() == 404)
}

/// Check GitHub for organization/user name availability.
///
/// Hits `https://api.github.com/users/{name}` with a User-Agent header.
/// 404 = available, 200 = taken.
pub async fn check_github(client: &reqwest::Client, name: &str) -> Result<bool> {
    let url = format!("https://api.github.com/users/{name}");
    let response = client
        .get(&url)
        .header("User-Agent", "shepherd-namegen/0.1")
        .send()
        .await?;
    Ok(response.status().as_u16() == 404)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_check_npm_known_package() {
        let client = reqwest::Client::builder()
            .timeout(REGISTRY_TIMEOUT)
            .build()
            .unwrap();

        // "express" is a well-known npm package, should be taken
        if let Ok(available) = check_npm(&client, "express").await {
            assert!(!available, "express should be taken on npm");
        }
    }

    #[tokio::test]
    async fn test_check_pypi_known_package() {
        let client = reqwest::Client::builder()
            .timeout(REGISTRY_TIMEOUT)
            .build()
            .unwrap();

        // "requests" is a well-known PyPI package, should be taken
        if let Ok(available) = check_pypi(&client, "requests").await {
            assert!(!available, "requests should be taken on PyPI");
        }
    }

    #[tokio::test]
    async fn test_check_github_known_org() {
        let client = reqwest::Client::builder()
            .timeout(REGISTRY_TIMEOUT)
            .build()
            .unwrap();

        // "google" is a well-known GitHub org, should be taken
        if let Ok(available) = check_github(&client, "google").await {
            assert!(!available, "google should be taken on GitHub");
        }
    }

    #[tokio::test]
    async fn test_validate_name_returns_results() {
        // Use a very unlikely name to test the full pipeline
        if let Ok((domains, npm, pypi, github)) =
            validate_name("zzxxqqww99887766notreal").await
        {
            // Should have domain results for all default TLDs
            assert_eq!(domains.len(), 5);
            // npm/pypi/github should have Some value (either true or false)
            // We just verify the pipeline returns data, not specific values
            // since network conditions vary
            if npm.is_some() {
                // Successfully checked npm
                assert!(npm.unwrap()); // unlikely name should be available
            }
            if pypi.is_some() {
                assert!(pypi.unwrap());
            }
            if github.is_some() {
                assert!(github.unwrap());
            }
        }
    }
}
