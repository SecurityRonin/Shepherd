use anyhow::Result;
use serde::Deserialize;
use std::time::Duration;

use crate::namegen::DomainCheck;

/// Default TLDs to check for each name candidate.
pub const DEFAULT_TLDS: &[&str] = &["com", "dev", "io", "app", "codes"];

/// RDAP bootstrap URL for DNS service discovery.
const RDAP_BOOTSTRAP_URL: &str = "https://data.iana.org/rdap/dns.json";

/// Fallback RDAP server when bootstrap lookup fails.
const FALLBACK_RDAP_SERVER: &str = "https://rdap.org";

/// Timeout for RDAP HTTP requests.
const RDAP_TIMEOUT: Duration = Duration::from_secs(10);

/// RDAP bootstrap response format.
#[derive(Debug, Deserialize)]
struct RdapBootstrap {
    services: Vec<Vec<serde_json::Value>>,
}

/// Result of checking a single domain via RDAP.
#[derive(Debug, Clone)]
pub struct DomainResult {
    pub domain: String,
    pub available: Option<bool>,
    pub error: Option<String>,
}

/// Check a single domain's availability via RDAP.
///
/// A 404 response indicates the domain is available (not registered).
/// A 200 response indicates the domain is registered (not available).
/// Any other response or error is reported in the error field.
pub async fn check_domain(client: &reqwest::Client, domain: &str) -> DomainResult {
    let tld = domain.rsplit('.').next().unwrap_or("");
    let server = match find_rdap_server(client, tld).await {
        Ok(s) => s,
        Err(e) => {
            return DomainResult {
                domain: domain.to_string(),
                available: None,
                error: Some(format!("Failed to find RDAP server: {e}")),
            };
        }
    };

    let url = format!("{}/domain/{}", server.trim_end_matches('/'), domain);
    match client.get(&url).send().await {
        Ok(resp) => {
            let status = resp.status().as_u16();
            match status {
                404 => DomainResult {
                    domain: domain.to_string(),
                    available: Some(true),
                    error: None,
                },
                200 => DomainResult {
                    domain: domain.to_string(),
                    available: Some(false),
                    error: None,
                },
                _ => DomainResult {
                    domain: domain.to_string(),
                    available: None,
                    error: Some(format!("Unexpected status: {status}")),
                },
            }
        }
        Err(e) => DomainResult {
            domain: domain.to_string(),
            available: None,
            error: Some(format!("Request failed: {e}")),
        },
    }
}

/// Check all default TLD domains for a given name.
pub async fn check_domains_for_name(name: &str) -> Vec<DomainCheck> {
    let client = reqwest::Client::builder()
        .timeout(RDAP_TIMEOUT)
        .build()
        .unwrap_or_default();

    let mut results = Vec::new();
    for tld in DEFAULT_TLDS {
        let domain = format!("{name}.{tld}");
        let result = check_domain(&client, &domain).await;
        results.push(DomainCheck {
            domain: result.domain,
            available: result.available,
            error: result.error,
        });
    }
    results
}

/// Find the RDAP server for a given TLD using IANA bootstrap data.
///
/// Falls back to the generic rdap.org server if bootstrap lookup fails.
pub async fn find_rdap_server(client: &reqwest::Client, tld: &str) -> Result<String> {
    let response = match client.get(RDAP_BOOTSTRAP_URL).send().await {
        Ok(r) => r,
        Err(_) => return Ok(FALLBACK_RDAP_SERVER.to_string()),
    };

    let bootstrap: RdapBootstrap = match response.json().await {
        Ok(b) => b,
        Err(_) => return Ok(FALLBACK_RDAP_SERVER.to_string()),
    };

    // Search through services for a match
    for service in &bootstrap.services {
        if service.len() < 2 {
            continue;
        }

        // First element is array of TLDs, second is array of server URLs
        if let (Some(tlds), Some(urls)) = (service[0].as_array(), service[1].as_array()) {
            for tld_value in tlds {
                if let Some(tld_str) = tld_value.as_str() {
                    if tld_str.eq_ignore_ascii_case(tld) {
                        // Return the first URL
                        if let Some(first_url) = urls.first().and_then(|u| u.as_str()) {
                            return Ok(first_url.to_string());
                        }
                    }
                }
            }
        }
    }

    // Fallback
    Ok(FALLBACK_RDAP_SERVER.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_tlds_count() {
        assert_eq!(DEFAULT_TLDS.len(), 5);
        assert!(DEFAULT_TLDS.contains(&"com"));
        assert!(DEFAULT_TLDS.contains(&"dev"));
        assert!(DEFAULT_TLDS.contains(&"io"));
        assert!(DEFAULT_TLDS.contains(&"app"));
        assert!(DEFAULT_TLDS.contains(&"codes"));
    }

    #[test]
    fn test_domain_result_available() {
        let result = DomainResult {
            domain: "test.com".to_string(),
            available: Some(true),
            error: None,
        };
        assert_eq!(result.available, Some(true));
        assert!(result.error.is_none());
    }

    #[test]
    fn test_domain_result_error() {
        let result = DomainResult {
            domain: "test.com".to_string(),
            available: None,
            error: Some("connection failed".to_string()),
        };
        assert!(result.available.is_none());
        assert!(result.error.is_some());
    }

    #[test]
    fn domain_result_all_fields() {
        let r = DomainResult {
            domain: "example.com".into(),
            available: Some(false),
            error: None,
        };
        assert_eq!(r.domain, "example.com");
        assert_eq!(r.available, Some(false));
        assert!(r.error.is_none());

        let r2 = DomainResult {
            domain: "fail.com".into(),
            available: None,
            error: Some("timeout".into()),
        };
        assert!(r2.available.is_none());
        assert_eq!(r2.error.as_deref(), Some("timeout"));
    }

    #[test]
    fn rdap_bootstrap_deserialize() {
        let json = r#"{"version":"1.0","services":[[["com"],["https://rdap.verisign.com/com/v1/"]],[["dev"],["https://rdap.nic.google/"]]]}"#;
        let bootstrap: RdapBootstrap = serde_json::from_str(json).unwrap();
        assert_eq!(bootstrap.services.len(), 2);
    }

    #[test]
    fn default_tlds_all_lowercase() {
        for tld in DEFAULT_TLDS {
            assert_eq!(*tld, tld.to_lowercase(), "TLD should be lowercase: {tld}");
        }
    }

    #[test]
    fn default_tlds_no_dots() {
        for tld in DEFAULT_TLDS {
            assert!(!tld.contains('.'), "TLD should not contain dots: {tld}");
        }
    }
}
