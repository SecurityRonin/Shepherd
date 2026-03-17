pub mod auth;
pub mod credits;
pub mod generation;
pub mod sync;
pub mod observability;
pub mod notifications;

use serde::{Deserialize, Serialize};
use std::fmt;

/// Base URL for the Shepherd Pro cloud API.
pub const DEFAULT_API_URL: &str = "https://api.shepherd.codes";

/// Credit costs for generative features.
pub const CREDIT_COST_LOGO: u32 = 2;
pub const CREDIT_COST_NAME: u32 = 1;
pub const CREDIT_COST_NORTHSTAR: u32 = 15;
pub const CREDIT_COST_SCRAPE: u32 = 1;
pub const CREDIT_COST_CRAWL: u32 = 5;
pub const CREDIT_COST_VISION: u32 = 2;
pub const CREDIT_COST_SEARCH: u32 = 1;

/// Number of free trials per generative feature.
pub const TRIAL_LIMIT: u32 = 2;

/// Monthly credits included with Free plan.
pub const FREE_MONTHLY_CREDITS: u32 = 2;

/// Monthly credits included with Pro subscription.
pub const PRO_MONTHLY_CREDITS: u32 = 100;

/// Monthly credits included with Pro Plus subscription.
pub const PRO_PLUS_MONTHLY_CREDITS: u32 = 300;

/// Credits included in a top-up purchase.
pub const TOPUP_CREDITS: u32 = 50;

/// User's subscription plan.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Plan {
    Free,
    Pro,
    ProPlus,
}

impl Default for Plan {
    fn default() -> Self {
        Self::Free
    }
}

impl fmt::Display for Plan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Plan::Free => write!(f, "free"),
            Plan::Pro => write!(f, "pro"),
            Plan::ProPlus => write!(f, "pro_plus"),
        }
    }
}

/// Trial usage counts per generative feature.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct TrialCounts {
    #[serde(default)]
    pub logo: u32,
    #[serde(default)]
    pub name: u32,
    #[serde(default)]
    pub northstar: u32,
    #[serde(default)]
    pub scrape: u32,
    #[serde(default)]
    pub crawl: u32,
    #[serde(default)]
    pub vision: u32,
    #[serde(default)]
    pub search: u32,
}

impl TrialCounts {
    /// Check if a trial is available for a given feature.
    pub fn has_trial(&self, feature: &str) -> bool {
        match feature {
            "logo" => self.logo < TRIAL_LIMIT,
            "name" => self.name < TRIAL_LIMIT,
            "northstar" => self.northstar < TRIAL_LIMIT,
            "scrape" => self.scrape < TRIAL_LIMIT,
            "crawl" => self.crawl < TRIAL_LIMIT,
            "vision" => self.vision < TRIAL_LIMIT,
            "search" => self.search < TRIAL_LIMIT,
            _ => false,
        }
    }

    /// Get remaining trials for a feature.
    pub fn remaining(&self, feature: &str) -> u32 {
        match feature {
            "logo" => TRIAL_LIMIT.saturating_sub(self.logo),
            "name" => TRIAL_LIMIT.saturating_sub(self.name),
            "northstar" => TRIAL_LIMIT.saturating_sub(self.northstar),
            "scrape" => TRIAL_LIMIT.saturating_sub(self.scrape),
            "crawl" => TRIAL_LIMIT.saturating_sub(self.crawl),
            "vision" => TRIAL_LIMIT.saturating_sub(self.vision),
            "search" => TRIAL_LIMIT.saturating_sub(self.search),
            _ => 0,
        }
    }
}

/// Cached user profile stored in ~/.shepherd/auth.toml.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedProfile {
    pub user_id: String,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub github_handle: Option<String>,
    #[serde(default)]
    pub plan: Plan,
    #[serde(default)]
    pub credits_balance: u32,
    #[serde(default)]
    pub trial_counts: TrialCounts,
}

/// API response for the /api/credits/balance endpoint.
#[derive(Debug, Clone, Deserialize)]
pub struct BalanceResponse {
    pub plan: Plan,
    pub credits_balance: u32,
    pub trial_logo: u32,
    pub trial_name: u32,
    pub trial_northstar: u32,
    #[serde(default)]
    pub trial_scrape: u32,
    #[serde(default)]
    pub trial_crawl: u32,
    #[serde(default)]
    pub trial_vision: u32,
    #[serde(default)]
    pub trial_search: u32,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub github_handle: Option<String>,
}

/// API error response from the cloud backend.
#[derive(Debug, Clone, Deserialize)]
pub struct ApiErrorResponse {
    pub error: String,
}

/// Cloud-specific error type.
#[derive(Debug)]
pub enum CloudError {
    /// Not authenticated — no JWT available.
    NotAuthenticated,
    /// JWT expired or invalid.
    AuthExpired,
    /// Insufficient credits for the requested operation.
    InsufficientCredits { required: u32, available: u32 },
    /// No trials remaining for this feature.
    NoTrialsRemaining { feature: String },
    /// Network error (offline or unreachable).
    Network(String),
    /// API returned an error.
    Api { status: u16, message: String },
    /// Keychain access error.
    Keychain(String),
}

impl fmt::Display for CloudError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CloudError::NotAuthenticated => write!(f, "Not signed in. Sign in to use cloud features."),
            CloudError::AuthExpired => write!(f, "Session expired. Please sign in again."),
            CloudError::InsufficientCredits { required, available } => {
                write!(f, "Insufficient credits: need {required}, have {available}")
            }
            CloudError::NoTrialsRemaining { feature } => {
                write!(f, "No free trials remaining for {feature}. Upgrade to Pro for credits.")
            }
            CloudError::Network(msg) => write!(f, "Network error: {msg}"),
            CloudError::Api { status, message } => write!(f, "API error ({status}): {message}"),
            CloudError::Keychain(msg) => write!(f, "Keychain error: {msg}"),
        }
    }
}

impl std::error::Error for CloudError {}

/// Configuration for the cloud client.
#[derive(Debug, Clone)]
pub struct CloudConfig {
    /// Base URL for the API (default: DEFAULT_API_URL).
    pub api_url: String,
}

impl Default for CloudConfig {
    fn default() -> Self {
        Self {
            api_url: DEFAULT_API_URL.to_string(),
        }
    }
}

/// HTTP client for communicating with the Shepherd Pro cloud backend.
#[derive(Debug, Clone)]
pub struct CloudClient {
    pub(crate) http: reqwest::Client,
    pub(crate) config: CloudConfig,
    #[cfg(test)]
    pub(crate) test_jwt: Option<String>,
}

impl CloudClient {
    /// Create a new cloud client with default configuration.
    pub fn new() -> Self {
        Self::with_config(CloudConfig::default())
    }

    /// Create a new cloud client with custom configuration.
    pub fn with_config(config: CloudConfig) -> Self {
        let http = reqwest::Client::builder()
            .user_agent("shepherd-desktop/1.0")
            .build()
            .expect("Failed to create HTTP client");
        Self {
            http,
            config,
            #[cfg(test)]
            test_jwt: None,
        }
    }

    /// Test-only constructor that injects a fake JWT and points at a mock server URL.
    #[cfg(test)]
    pub fn with_test_jwt(api_url: &str, jwt: &str) -> Self {
        let mut client = Self::with_config(CloudConfig { api_url: api_url.to_string() });
        client.test_jwt = Some(jwt.to_string());
        client
    }

    /// Get the base API URL.
    pub fn api_url(&self) -> &str {
        &self.config.api_url
    }

    /// Extract JWT, encapsulating the test/prod conditional.
    pub(crate) fn get_jwt(&self) -> Result<String, CloudError> {
        #[cfg(test)]
        { self.test_jwt.clone().ok_or(CloudError::NotAuthenticated) }
        #[cfg(not(test))]
        { auth::load_jwt().ok_or(CloudError::NotAuthenticated) }
    }
}

impl Default for CloudClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_default_is_free() {
        assert_eq!(Plan::default(), Plan::Free);
    }

    #[test]
    fn plan_display() {
        assert_eq!(Plan::Free.to_string(), "free");
        assert_eq!(Plan::Pro.to_string(), "pro");
    }

    #[test]
    fn plan_serde_roundtrip() {
        let json = serde_json::to_string(&Plan::Pro).unwrap();
        assert_eq!(json, "\"pro\"");
        let parsed: Plan = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, Plan::Pro);
    }

    #[test]
    fn trial_counts_has_trial() {
        let counts = TrialCounts {
            logo: 0, name: 1, northstar: 2,
            scrape: 0, crawl: 1, vision: 2, search: 1,
        };
        assert!(counts.has_trial("logo"));
        assert!(counts.has_trial("name"));
        assert!(!counts.has_trial("northstar")); // 2 >= TRIAL_LIMIT
        assert!(counts.has_trial("scrape"));
        assert!(counts.has_trial("crawl"));
        assert!(!counts.has_trial("vision")); // 2 >= TRIAL_LIMIT
        assert!(counts.has_trial("search"));
        assert!(!counts.has_trial("unknown"));
    }

    #[test]
    fn trial_counts_remaining() {
        let counts = TrialCounts {
            logo: 0, name: 1, northstar: 2,
            scrape: 0, crawl: 1, vision: 2, search: 1,
        };
        assert_eq!(counts.remaining("logo"), 2);
        assert_eq!(counts.remaining("name"), 1);
        assert_eq!(counts.remaining("northstar"), 0);
        assert_eq!(counts.remaining("scrape"), 2);
        assert_eq!(counts.remaining("crawl"), 1);
        assert_eq!(counts.remaining("vision"), 0);
        assert_eq!(counts.remaining("search"), 1);
        assert_eq!(counts.remaining("unknown"), 0);
    }

    #[test]
    fn trial_counts_default() {
        let counts = TrialCounts::default();
        assert_eq!(counts.logo, 0);
        assert_eq!(counts.name, 0);
        assert_eq!(counts.northstar, 0);
        assert_eq!(counts.scrape, 0);
        assert_eq!(counts.crawl, 0);
        assert_eq!(counts.vision, 0);
        assert_eq!(counts.search, 0);
        assert!(counts.has_trial("logo"));
        assert!(counts.has_trial("name"));
        assert!(counts.has_trial("northstar"));
        assert!(counts.has_trial("scrape"));
        assert!(counts.has_trial("crawl"));
        assert!(counts.has_trial("vision"));
        assert!(counts.has_trial("search"));
    }

    #[test]
    fn credit_costs() {
        assert_eq!(CREDIT_COST_LOGO, 2);
        assert_eq!(CREDIT_COST_NAME, 1);
        assert_eq!(CREDIT_COST_NORTHSTAR, 15);
        assert_eq!(CREDIT_COST_SCRAPE, 1);
        assert_eq!(CREDIT_COST_CRAWL, 5);
        assert_eq!(CREDIT_COST_VISION, 2);
        assert_eq!(CREDIT_COST_SEARCH, 1);
        assert_eq!(TRIAL_LIMIT, 2);
        assert_eq!(PRO_MONTHLY_CREDITS, 100);
        assert_eq!(TOPUP_CREDITS, 50);
    }

    #[test]
    fn cloud_client_default_url() {
        let client = CloudClient::new();
        assert_eq!(client.api_url(), DEFAULT_API_URL);
    }

    #[test]
    fn cloud_client_custom_url() {
        let config = CloudConfig {
            api_url: "http://localhost:3000".to_string(),
        };
        let client = CloudClient::with_config(config);
        assert_eq!(client.api_url(), "http://localhost:3000");
    }

    #[test]
    fn cloud_error_display() {
        let err = CloudError::NotAuthenticated;
        assert!(err.to_string().contains("Not signed in"));

        let err = CloudError::InsufficientCredits { required: 15, available: 3 };
        assert!(err.to_string().contains("15"));
        assert!(err.to_string().contains("3"));

        let err = CloudError::NoTrialsRemaining { feature: "logo".to_string() };
        assert!(err.to_string().contains("logo"));
    }

    #[test]
    fn cached_profile_serde() {
        let profile = CachedProfile {
            user_id: "user-123".to_string(),
            email: Some("test@example.com".to_string()),
            github_handle: Some("testuser".to_string()),
            plan: Plan::Pro,
            credits_balance: 42,
            trial_counts: TrialCounts {
                logo: 1, name: 0, northstar: 2,
                scrape: 1, crawl: 0, vision: 2, search: 0,
            },
        };

        let toml_str = toml::to_string_pretty(&profile).unwrap();
        let parsed: CachedProfile = toml::from_str(&toml_str).unwrap();

        assert_eq!(parsed.user_id, "user-123");
        assert_eq!(parsed.plan, Plan::Pro);
        assert_eq!(parsed.credits_balance, 42);
        assert_eq!(parsed.trial_counts.logo, 1);
        assert_eq!(parsed.trial_counts.scrape, 1);
        assert_eq!(parsed.trial_counts.crawl, 0);
        assert_eq!(parsed.trial_counts.vision, 2);
    }

    #[test]
    fn balance_response_deserialize() {
        let json = r#"{
            "plan": "pro",
            "credits_balance": 50,
            "trial_logo": 0,
            "trial_name": 1,
            "trial_northstar": 2,
            "trial_scrape": 1,
            "trial_crawl": 0,
            "trial_vision": 2,
            "email": "user@example.com",
            "github_handle": "gh-user"
        }"#;

        let resp: BalanceResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.plan, Plan::Pro);
        assert_eq!(resp.credits_balance, 50);
        assert_eq!(resp.trial_logo, 0);
        assert_eq!(resp.trial_name, 1);
        assert_eq!(resp.trial_northstar, 2);
        assert_eq!(resp.trial_scrape, 1);
        assert_eq!(resp.trial_crawl, 0);
        assert_eq!(resp.trial_vision, 2);
        assert_eq!(resp.email, Some("user@example.com".to_string()));
    }

    #[test]
    fn balance_response_deserialize_without_new_fields() {
        // Ensure backward compatibility: old responses without scrape/crawl/vision
        let json = r#"{
            "plan": "free",
            "credits_balance": 0,
            "trial_logo": 0,
            "trial_name": 0,
            "trial_northstar": 0
        }"#;

        let resp: BalanceResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.trial_scrape, 0);
        assert_eq!(resp.trial_crawl, 0);
        assert_eq!(resp.trial_vision, 0);
    }

    #[test]
    fn cloud_error_display_auth_expired() {
        let err = CloudError::AuthExpired;
        assert!(err.to_string().contains("expired"));
        assert!(err.to_string().contains("sign in"));
    }

    #[test]
    fn cloud_error_display_network() {
        let err = CloudError::Network("connection refused".to_string());
        let msg = err.to_string();
        assert!(msg.contains("Network error"));
        assert!(msg.contains("connection refused"));
    }

    #[test]
    fn cloud_error_display_api() {
        let err = CloudError::Api { status: 500, message: "Internal Server Error".to_string() };
        let msg = err.to_string();
        assert!(msg.contains("500"));
        assert!(msg.contains("Internal Server Error"));
    }

    #[test]
    fn cloud_error_display_keychain() {
        let err = CloudError::Keychain("permission denied".to_string());
        let msg = err.to_string();
        assert!(msg.contains("Keychain error"));
        assert!(msg.contains("permission denied"));
    }

    #[test]
    fn cloud_error_is_error_trait() {
        let err = CloudError::NotAuthenticated;
        let _: &dyn std::error::Error = &err;
    }

    #[test]
    fn cloud_config_default() {
        let config = CloudConfig::default();
        assert_eq!(config.api_url, DEFAULT_API_URL);
    }

    #[test]
    fn cloud_client_default_trait() {
        let client = CloudClient::default();
        assert_eq!(client.api_url(), DEFAULT_API_URL);
    }

    #[test]
    fn trial_counts_serde_roundtrip() {
        let counts = TrialCounts {
            logo: 1, name: 2, northstar: 0,
            scrape: 1, crawl: 0, vision: 2, search: 1,
        };
        let json = serde_json::to_string(&counts).unwrap();
        let parsed: TrialCounts = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.logo, 1);
        assert_eq!(parsed.vision, 2);
    }

    #[test]
    fn balance_response_with_search_field() {
        let json = r#"{
            "plan": "pro",
            "credits_balance": 50,
            "trial_logo": 0,
            "trial_name": 0,
            "trial_northstar": 0,
            "trial_scrape": 0,
            "trial_crawl": 0,
            "trial_vision": 0,
            "trial_search": 3
        }"#;
        let resp: BalanceResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.trial_search, 3);
    }

    #[test]
    fn plan_pro_plus_display() {
        assert_eq!(Plan::ProPlus.to_string(), "pro_plus");
    }

    #[test]
    fn plan_pro_plus_serde() {
        let json = serde_json::to_string(&Plan::ProPlus).unwrap();
        assert_eq!(json, "\"pro_plus\"");
        let parsed: Plan = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, Plan::ProPlus);
    }

    #[test]
    fn free_monthly_credits_constant() {
        assert_eq!(FREE_MONTHLY_CREDITS, 2);
    }

    #[test]
    fn pro_monthly_credits_updated() {
        assert_eq!(PRO_MONTHLY_CREDITS, 100);
    }

    #[test]
    fn pro_plus_monthly_credits_constant() {
        assert_eq!(PRO_PLUS_MONTHLY_CREDITS, 300);
    }

    #[test]
    fn topup_credits_updated() {
        assert_eq!(TOPUP_CREDITS, 50);
    }
}
