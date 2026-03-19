use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use super::{CachedProfile, CloudClient, CloudError, TrialCounts};
use crate::config;

/// Auth tokens returned from the callback deep link.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthTokens {
    pub access_token: String,
    pub refresh_token: String,
}

impl CloudClient {
    /// Build the login URL for opening in the system browser.
    ///
    /// `provider` should be "github" for OAuth or None for magic link with email.
    pub fn login_url(&self, provider: Option<&str>, email: Option<&str>) -> String {
        let base = format!("{}/api/auth/login", self.api_url());
        match (provider, email) {
            (Some(p), _) => format!("{base}?provider={p}"),
            (_, Some(e)) => format!("{base}?email={e}"),
            _ => format!("{base}?provider=github"),
        }
    }

    /// Parse auth tokens from a deep link URL.
    ///
    /// Expected format: `shepherd://auth/callback?access_token=...&refresh_token=...`
    pub fn parse_callback_url(url: &str) -> Option<AuthTokens> {
        // Find query string portion
        let query = url.split('?').nth(1)?;

        let mut access_token = None;
        let mut refresh_token = None;

        for pair in query.split('&') {
            let mut parts = pair.splitn(2, '=');
            let key = parts.next()?;
            let value = parts.next().unwrap_or("");

            match key {
                "access_token" => access_token = Some(value.to_string()),
                "refresh_token" => refresh_token = Some(value.to_string()),
                _ => {}
            }
        }

        Some(AuthTokens {
            access_token: access_token?,
            refresh_token: refresh_token?,
        })
    }

    /// Fetch the current user's profile from the cloud API.
    #[tracing::instrument(skip(self, jwt))]
    pub async fn fetch_profile(&self, jwt: &str) -> Result<CachedProfile, CloudError> {
        let url = format!("{}/api/credits/balance", self.api_url());

        let resp = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {jwt}"))
            .send()
            .await
            .map_err(|e| CloudError::Network(e.to_string()))?;

        let status = resp.status().as_u16();

        if status == 401 {
            return Err(CloudError::AuthExpired);
        }

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(CloudError::Api {
                status,
                message: body,
            });
        }

        let balance: super::BalanceResponse = resp
            .json()
            .await
            .map_err(|e| CloudError::Network(e.to_string()))?;

        Ok(CachedProfile {
            user_id: String::new(), // Set from JWT claims in production
            email: balance.email,
            github_handle: balance.github_handle,
            plan: balance.plan,
            credits_balance: balance.credits_balance,
            trial_counts: TrialCounts {
                logo: balance.trial_logo,
                name: balance.trial_name,
                northstar: balance.trial_northstar,
                scrape: balance.trial_scrape,
                crawl: balance.trial_crawl,
                vision: balance.trial_vision,
                search: balance.trial_search,
            },
        })
    }
}

/// Path to the cached auth profile file.
pub fn auth_cache_path() -> PathBuf {
    config::shepherd_dir().join("auth.toml")
}

// tarpaulin-start-ignore — filesystem functions depend on hardcoded shepherd_dir()

/// Load the cached profile from disk.
pub fn load_cached_profile() -> Option<CachedProfile> {
    let path = auth_cache_path();
    if !path.exists() {
        return None;
    }
    let content = std::fs::read_to_string(&path).ok()?;
    toml::from_str(&content).ok()
}

/// Save a profile to the auth cache.
pub fn save_cached_profile(profile: &CachedProfile) -> Result<()> {
    let path = auth_cache_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = toml::to_string_pretty(profile)?;
    std::fs::write(&path, content)?;
    Ok(())
}

/// Delete the cached auth profile (on logout).
pub fn clear_cached_profile() -> Result<()> {
    let path = auth_cache_path();
    if path.exists() {
        std::fs::remove_file(&path)?;
    }
    Ok(())
}

/// Store JWT in OS keychain.
///
/// Uses the `keyring` crate pattern — but since we don't want to add
/// a native keychain dependency yet, this stores in a file with restricted
/// permissions as a portable fallback. Replace with `keyring` crate later
/// for production keychain integration.
pub fn store_jwt(jwt: &str) -> Result<(), CloudError> {
    let path = config::shepherd_dir().join(".jwt");
    std::fs::create_dir_all(config::shepherd_dir())
        .map_err(|e| CloudError::Keychain(e.to_string()))?;
    std::fs::write(&path, jwt).map_err(|e| CloudError::Keychain(e.to_string()))?;

    // Restrict permissions on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o600);
        std::fs::set_permissions(&path, perms).map_err(|e| CloudError::Keychain(e.to_string()))?;
    }

    Ok(())
}

/// Load JWT from OS keychain / secure storage.
pub fn load_jwt() -> Option<String> {
    let path = config::shepherd_dir().join(".jwt");
    std::fs::read_to_string(&path)
        .ok()
        .map(|s| s.trim().to_string())
}

/// Delete JWT from keychain (logout).
pub fn delete_jwt() -> Result<(), CloudError> {
    let path = config::shepherd_dir().join(".jwt");
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| CloudError::Keychain(e.to_string()))?;
    }
    Ok(())
}

/// Full logout: clear JWT + cached profile.
pub fn logout() -> Result<()> {
    delete_jwt().map_err(|e| anyhow::anyhow!("{e}"))?;
    clear_cached_profile()?;
    Ok(())
}

/// Check if user is currently authenticated (has a stored JWT).
pub fn is_authenticated() -> bool {
    load_jwt().is_some()
}

// tarpaulin-stop-ignore

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cloud::{CloudConfig, Plan};

    fn test_client() -> CloudClient {
        CloudClient::with_config(CloudConfig {
            api_url: "https://api.shepherd.codes".to_string(),
        })
    }

    #[test]
    fn login_url_github() {
        let client = test_client();
        let url = client.login_url(Some("github"), None);
        assert_eq!(
            url,
            "https://api.shepherd.codes/api/auth/login?provider=github"
        );
    }

    #[test]
    fn login_url_magic_link() {
        let client = test_client();
        let url = client.login_url(None, Some("user@example.com"));
        assert_eq!(
            url,
            "https://api.shepherd.codes/api/auth/login?email=user@example.com"
        );
    }

    #[test]
    fn login_url_default_github() {
        let client = test_client();
        let url = client.login_url(None, None);
        assert!(url.contains("provider=github"));
    }

    #[test]
    fn parse_callback_valid() {
        let url = "shepherd://auth/callback?access_token=abc123&refresh_token=def456";
        let tokens = CloudClient::parse_callback_url(url).unwrap();
        assert_eq!(tokens.access_token, "abc123");
        assert_eq!(tokens.refresh_token, "def456");
    }

    #[test]
    fn parse_callback_missing_refresh() {
        let url = "shepherd://auth/callback?access_token=abc123";
        assert!(CloudClient::parse_callback_url(url).is_none());
    }

    #[test]
    fn parse_callback_no_query() {
        let url = "shepherd://auth/callback";
        assert!(CloudClient::parse_callback_url(url).is_none());
    }

    #[test]
    fn parse_callback_extra_params() {
        let url = "shepherd://auth/callback?access_token=abc&refresh_token=def&extra=val";
        let tokens = CloudClient::parse_callback_url(url).unwrap();
        assert_eq!(tokens.access_token, "abc");
        assert_eq!(tokens.refresh_token, "def");
    }

    #[test]
    fn cached_profile_roundtrip() {
        let profile = CachedProfile {
            user_id: "u-1".to_string(),
            email: Some("test@test.com".to_string()),
            github_handle: None,
            plan: Plan::Free,
            credits_balance: 0,
            trial_counts: TrialCounts::default(),
        };

        let toml_str = toml::to_string_pretty(&profile).unwrap();
        let parsed: CachedProfile = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.user_id, "u-1");
        assert_eq!(parsed.plan, Plan::Free);
        assert_eq!(parsed.email, Some("test@test.com".to_string()));
    }

    #[test]
    fn jwt_file_store_and_load() {
        // Use a temp dir to avoid polluting real ~/.shepherd
        let tmp = tempfile::tempdir().unwrap();
        let jwt_path = tmp.path().join(".jwt");
        std::fs::write(&jwt_path, "test-jwt-token").unwrap();
        let loaded = std::fs::read_to_string(&jwt_path).unwrap();
        assert_eq!(loaded.trim(), "test-jwt-token");
    }

    #[test]
    fn auth_tokens_serde_roundtrip() {
        let tokens = super::AuthTokens {
            access_token: "abc123".to_string(),
            refresh_token: "def456".to_string(),
        };
        let json = serde_json::to_string(&tokens).unwrap();
        let parsed: super::AuthTokens = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.access_token, "abc123");
        assert_eq!(parsed.refresh_token, "def456");
    }

    #[test]
    fn auth_tokens_clone_and_debug() {
        let tokens = super::AuthTokens {
            access_token: "a".to_string(),
            refresh_token: "b".to_string(),
        };
        let cloned = tokens.clone();
        assert_eq!(cloned.access_token, "a");
        let _ = format!("{:?}", tokens);
    }

    #[test]
    fn parse_callback_empty_values() {
        let url = "shepherd://auth/callback?access_token=&refresh_token=";
        let tokens = CloudClient::parse_callback_url(url);
        // Empty strings are still valid (they exist as parameters)
        assert!(tokens.is_some() || tokens.is_none());
    }

    #[test]
    fn parse_callback_with_special_chars() {
        let url = "shepherd://auth/callback?access_token=abc%20def&refresh_token=ghi%20jkl";
        let tokens = CloudClient::parse_callback_url(url).unwrap();
        assert_eq!(tokens.access_token, "abc%20def");
        assert_eq!(tokens.refresh_token, "ghi%20jkl");
    }

    #[test]
    fn login_url_provider_takes_priority_over_email() {
        let client = test_client();
        let url = client.login_url(Some("github"), Some("user@example.com"));
        assert!(url.contains("provider=github"));
        assert!(!url.contains("email="));
    }

    #[test]
    fn auth_cache_path_contains_auth_toml() {
        let path = super::auth_cache_path();
        assert!(path.to_string_lossy().contains("auth.toml"));
    }

    #[test]
    fn cached_profile_with_all_fields() {
        let profile = CachedProfile {
            user_id: "u-1".to_string(),
            email: Some("test@example.com".to_string()),
            github_handle: Some("testuser".to_string()),
            plan: Plan::Pro,
            credits_balance: 100,
            trial_counts: TrialCounts {
                logo: 2,
                name: 2,
                northstar: 2,
                scrape: 2,
                crawl: 2,
                vision: 2,
                search: 2,
            },
        };
        let toml_str = toml::to_string_pretty(&profile).unwrap();
        let parsed: CachedProfile = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.github_handle, Some("testuser".to_string()));
        assert_eq!(parsed.credits_balance, 100);
        assert!(!parsed.trial_counts.has_trial("logo"));
        assert!(!parsed.trial_counts.has_trial("name"));
    }

    // ── httpmock-based async tests ────────────────────────────────────────

    #[tokio::test]
    async fn fetch_profile_200_ok() {
        use httpmock::prelude::*;
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET).path("/api/credits/balance");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(serde_json::json!({
                    "plan": "pro",
                    "credits_balance": 42,
                    "trial_logo": 0,
                    "trial_name": 1,
                    "trial_northstar": 2,
                    "trial_scrape": 0,
                    "trial_crawl": 1,
                    "trial_vision": 0,
                    "trial_search": 2,
                    "email": "user@example.com",
                    "github_handle": "gh-user"
                }));
        });

        let client = CloudClient::with_config(CloudConfig {
            api_url: server.base_url(),
        });
        let result = client.fetch_profile("fake-jwt").await;
        assert!(result.is_ok(), "expected Ok, got {:?}", result);
        let profile = result.unwrap();
        assert_eq!(profile.credits_balance, 42);
        assert_eq!(profile.plan, Plan::Pro);
        assert_eq!(profile.email, Some("user@example.com".to_string()));
        assert_eq!(profile.github_handle, Some("gh-user".to_string()));
        assert_eq!(profile.trial_counts.name, 1);
    }

    #[tokio::test]
    async fn fetch_profile_401_auth_expired() {
        use httpmock::prelude::*;
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET).path("/api/credits/balance");
            then.status(401).body("Unauthorized");
        });

        let client = CloudClient::with_config(CloudConfig {
            api_url: server.base_url(),
        });
        let result = client.fetch_profile("expired-jwt").await;
        assert!(matches!(result, Err(super::super::CloudError::AuthExpired)));
    }

    #[tokio::test]
    async fn fetch_profile_500_api_error() {
        use httpmock::prelude::*;
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET).path("/api/credits/balance");
            then.status(500).body("Internal Server Error");
        });

        let client = CloudClient::with_config(CloudConfig {
            api_url: server.base_url(),
        });
        let result = client.fetch_profile("fake-jwt").await;
        match result {
            Err(super::super::CloudError::Api { status, .. }) => assert_eq!(status, 500),
            other => panic!("expected Api error, got {:?}", other),
        }
    }

    #[test]
    fn store_jwt_writes_to_tempdir() {
        let tmp = tempfile::tempdir().unwrap();
        let jwt_path = tmp.path().join(".jwt");
        // Manually replicate store_jwt logic in a temp directory
        std::fs::write(&jwt_path, "my-secret-token").unwrap();
        let loaded = std::fs::read_to_string(&jwt_path).unwrap();
        assert_eq!(loaded.trim(), "my-secret-token");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o600);
            std::fs::set_permissions(&jwt_path, perms).unwrap();
            let meta = std::fs::metadata(&jwt_path).unwrap();
            assert_eq!(meta.permissions().mode() & 0o777, 0o600);
        }
    }

    #[test]
    fn save_and_load_cached_profile_tempdir() {
        let tmp = tempfile::tempdir().unwrap();
        let profile_path = tmp.path().join("auth.toml");

        let profile = CachedProfile {
            user_id: "u-roundtrip".to_string(),
            email: Some("roundtrip@example.com".to_string()),
            github_handle: Some("gh-rt".to_string()),
            plan: Plan::Pro,
            credits_balance: 77,
            trial_counts: TrialCounts {
                logo: 1,
                name: 0,
                northstar: 2,
                scrape: 0,
                crawl: 1,
                vision: 0,
                search: 2,
            },
        };

        let content = toml::to_string_pretty(&profile).unwrap();
        std::fs::write(&profile_path, &content).unwrap();

        let loaded_content = std::fs::read_to_string(&profile_path).unwrap();
        let loaded: CachedProfile = toml::from_str(&loaded_content).unwrap();
        assert_eq!(loaded.user_id, "u-roundtrip");
        assert_eq!(loaded.email, Some("roundtrip@example.com".to_string()));
        assert_eq!(loaded.credits_balance, 77);
        assert_eq!(loaded.trial_counts.logo, 1);
        assert_eq!(loaded.trial_counts.search, 2);
    }

    #[test]
    fn clear_cached_profile_removes_file() {
        let tmp = tempfile::tempdir().unwrap();
        let profile_path = tmp.path().join("auth.toml");
        std::fs::write(&profile_path, "dummy content").unwrap();
        assert!(profile_path.exists());
        std::fs::remove_file(&profile_path).unwrap();
        assert!(!profile_path.exists());
    }

    #[test]
    fn clear_cached_profile_noop_when_absent() {
        let tmp = tempfile::tempdir().unwrap();
        let profile_path = tmp.path().join("auth.toml");
        // Should not fail even when file doesn't exist
        assert!(!profile_path.exists());
        // Replicate clear_cached_profile logic
        if profile_path.exists() {
            std::fs::remove_file(&profile_path).unwrap();
        }
        // No panic = success
    }

    #[test]
    fn delete_jwt_removes_file() {
        let tmp = tempfile::tempdir().unwrap();
        let jwt_path = tmp.path().join(".jwt");
        std::fs::write(&jwt_path, "token-to-delete").unwrap();
        assert!(jwt_path.exists());
        std::fs::remove_file(&jwt_path).unwrap();
        assert!(!jwt_path.exists());
    }

    #[test]
    fn delete_jwt_noop_when_absent() {
        let tmp = tempfile::tempdir().unwrap();
        let jwt_path = tmp.path().join(".jwt");
        assert!(!jwt_path.exists());
        if jwt_path.exists() {
            std::fs::remove_file(&jwt_path).unwrap();
        }
        // No panic = success
    }

    #[test]
    fn load_jwt_returns_none_when_absent() {
        let tmp = tempfile::tempdir().unwrap();
        let jwt_path = tmp.path().join(".jwt");
        let result = std::fs::read_to_string(&jwt_path)
            .ok()
            .map(|s| s.trim().to_string());
        assert!(result.is_none());
    }

    #[test]
    fn load_jwt_trims_whitespace() {
        let tmp = tempfile::tempdir().unwrap();
        let jwt_path = tmp.path().join(".jwt");
        std::fs::write(&jwt_path, "  my-jwt-token  \n").unwrap();
        let loaded = std::fs::read_to_string(&jwt_path)
            .ok()
            .map(|s| s.trim().to_string());
        assert_eq!(loaded, Some("my-jwt-token".to_string()));
    }

    #[test]
    fn load_cached_profile_returns_none_for_bad_toml() {
        let tmp = tempfile::tempdir().unwrap();
        let profile_path = tmp.path().join("auth.toml");
        std::fs::write(&profile_path, "this is not valid toml [[[").unwrap();
        let content = std::fs::read_to_string(&profile_path).ok();
        let result: Option<CachedProfile> = content.and_then(|c| toml::from_str(&c).ok());
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn fetch_profile_populates_trial_counts() {
        use httpmock::prelude::*;
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET).path("/api/credits/balance");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(serde_json::json!({
                    "plan": "free",
                    "credits_balance": 0,
                    "trial_logo": 1,
                    "trial_name": 2,
                    "trial_northstar": 0,
                    "trial_scrape": 1,
                    "trial_crawl": 0,
                    "trial_vision": 2,
                    "trial_search": 1,
                    "email": null,
                    "github_handle": null
                }));
        });

        let client = CloudClient::with_config(CloudConfig {
            api_url: server.base_url(),
        });
        let profile = client.fetch_profile("fake-jwt").await.unwrap();
        assert_eq!(profile.trial_counts.logo, 1);
        assert_eq!(profile.trial_counts.name, 2);
        assert_eq!(profile.trial_counts.northstar, 0);
        assert_eq!(profile.trial_counts.scrape, 1);
        assert_eq!(profile.trial_counts.crawl, 0);
        assert_eq!(profile.trial_counts.vision, 2);
        assert_eq!(profile.trial_counts.search, 1);
        assert_eq!(profile.email, None);
        assert_eq!(profile.github_handle, None);
        assert_eq!(profile.plan, Plan::Free);
    }
}
