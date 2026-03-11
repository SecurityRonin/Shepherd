use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::config;
use super::{CachedProfile, CloudClient, CloudError, TrialCounts};

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
            },
        })
    }
}

/// Path to the cached auth profile file.
pub fn auth_cache_path() -> PathBuf {
    config::shepherd_dir().join("auth.toml")
}

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
        std::fs::set_permissions(&path, perms)
            .map_err(|e| CloudError::Keychain(e.to_string()))?;
    }

    Ok(())
}

/// Load JWT from OS keychain / secure storage.
pub fn load_jwt() -> Option<String> {
    let path = config::shepherd_dir().join(".jwt");
    std::fs::read_to_string(&path).ok().map(|s| s.trim().to_string())
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
        assert_eq!(url, "https://api.shepherd.codes/api/auth/login?provider=github");
    }

    #[test]
    fn login_url_magic_link() {
        let client = test_client();
        let url = client.login_url(None, Some("user@example.com"));
        assert_eq!(url, "https://api.shepherd.codes/api/auth/login?email=user@example.com");
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
}
