use super::{
    auth, CloudClient, CloudError,
    CREDIT_COST_LOGO, CREDIT_COST_NAME, CREDIT_COST_NORTHSTAR,
    CREDIT_COST_SCRAPE, CREDIT_COST_CRAWL, CREDIT_COST_VISION,
    CREDIT_COST_SEARCH,
};

/// Feature identifier for credit/trial operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Feature {
    Logo,
    Name,
    NorthStar,
    Scrape,
    Crawl,
    Vision,
    Search,
}

impl Feature {
    /// Credit cost for this feature.
    pub fn cost(&self) -> u32 {
        match self {
            Feature::Logo => CREDIT_COST_LOGO,
            Feature::Name => CREDIT_COST_NAME,
            Feature::NorthStar => CREDIT_COST_NORTHSTAR,
            Feature::Scrape => CREDIT_COST_SCRAPE,
            Feature::Crawl => CREDIT_COST_CRAWL,
            Feature::Vision => CREDIT_COST_VISION,
            Feature::Search => CREDIT_COST_SEARCH,
        }
    }

    /// Feature key string used in API calls and trial tracking.
    pub fn key(&self) -> &str {
        match self {
            Feature::Logo => "logo",
            Feature::Name => "name",
            Feature::NorthStar => "northstar",
            Feature::Scrape => "scrape",
            Feature::Crawl => "crawl",
            Feature::Vision => "vision",
            Feature::Search => "search",
        }
    }
}

impl std::fmt::Display for Feature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.key())
    }
}

/// Whether a generative feature can be used.
#[derive(Debug, PartialEq, Eq)]
pub enum AccessCheck {
    /// Has credits to use.
    HasCredits(u32),
    /// Has a free trial available.
    HasTrial(u32),
    /// Not authenticated.
    NotAuthenticated,
    /// No credits or trials remaining.
    NeedsUpgrade,
}

impl CloudClient {
    /// Check if the user can use a generative feature.
    ///
    /// Checks cached profile first (offline-capable for display purposes).
    /// Actual enforcement happens server-side.
    pub fn check_access(&self, feature: Feature) -> AccessCheck {
        if !auth::is_authenticated() {
            return AccessCheck::NotAuthenticated;
        }

        let profile = match auth::load_cached_profile() {
            Some(p) => p,
            None => return AccessCheck::NotAuthenticated,
        };

        // Check trial first
        let remaining_trials = profile.trial_counts.remaining(feature.key());
        if remaining_trials > 0 {
            return AccessCheck::HasTrial(remaining_trials);
        }

        // Check credits
        if profile.credits_balance >= feature.cost() {
            return AccessCheck::HasCredits(profile.credits_balance);
        }

        AccessCheck::NeedsUpgrade
    }

    /// Build the Stripe Checkout URL for a subscription purchase.
    pub fn subscription_url(&self) -> String {
        format!("{}/api/credits/purchase", self.api_url())
    }

    /// Build the Stripe Checkout URL for a credit top-up.
    pub fn topup_url(&self) -> String {
        format!("{}/api/credits/purchase", self.api_url())
    }

    /// Refresh the cached credit balance from the server.
    #[tracing::instrument(skip(self))]
    pub async fn refresh_balance(&self) -> Result<u32, CloudError> {
        let jwt = auth::load_jwt().ok_or(CloudError::NotAuthenticated)?;
        let profile = self.fetch_profile(&jwt).await?;

        auth::save_cached_profile(&profile)
            .map_err(|e| CloudError::Keychain(e.to_string()))?;

        Ok(profile.credits_balance)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feature_costs() {
        assert_eq!(Feature::Logo.cost(), 2);
        assert_eq!(Feature::Name.cost(), 1);
        assert_eq!(Feature::NorthStar.cost(), 15);
        assert_eq!(Feature::Scrape.cost(), 1);
        assert_eq!(Feature::Crawl.cost(), 5);
        assert_eq!(Feature::Vision.cost(), 2);
        assert_eq!(Feature::Search.cost(), 1);
    }

    #[test]
    fn feature_keys() {
        assert_eq!(Feature::Logo.key(), "logo");
        assert_eq!(Feature::Name.key(), "name");
        assert_eq!(Feature::NorthStar.key(), "northstar");
        assert_eq!(Feature::Scrape.key(), "scrape");
        assert_eq!(Feature::Crawl.key(), "crawl");
        assert_eq!(Feature::Vision.key(), "vision");
        assert_eq!(Feature::Search.key(), "search");
    }

    #[test]
    fn feature_display() {
        assert_eq!(Feature::Logo.to_string(), "logo");
        assert_eq!(Feature::Name.to_string(), "name");
        assert_eq!(Feature::NorthStar.to_string(), "northstar");
        assert_eq!(Feature::Scrape.to_string(), "scrape");
        assert_eq!(Feature::Crawl.to_string(), "crawl");
        assert_eq!(Feature::Vision.to_string(), "vision");
        assert_eq!(Feature::Search.to_string(), "search");
    }

    #[test]
    fn access_check_not_authenticated() {
        // Without any stored JWT, check_access should return NotAuthenticated.
        // (In tests, is_authenticated() returns false since there's no ~/.shepherd/.jwt)
        let client = CloudClient::new();
        assert_eq!(client.check_access(Feature::Logo), AccessCheck::NotAuthenticated);
    }

    #[test]
    fn purchase_urls() {
        let client = CloudClient::new();
        assert!(client.subscription_url().contains("/api/credits/purchase"));
        assert!(client.topup_url().contains("/api/credits/purchase"));
    }

    #[test]
    fn feature_debug_and_clone() {
        let f = Feature::Logo;
        let f2 = f.clone();
        assert_eq!(f, f2);
        let _ = format!("{:?}", f);
    }

    #[test]
    fn feature_copy() {
        let f = Feature::NorthStar;
        let f2 = f;
        assert_eq!(f, f2);
    }

    #[test]
    fn access_check_debug() {
        let ac = AccessCheck::HasCredits(42);
        let _ = format!("{:?}", ac);
    }

    #[test]
    fn access_check_variants() {
        assert_eq!(AccessCheck::HasCredits(10), AccessCheck::HasCredits(10));
        assert_ne!(AccessCheck::HasCredits(10), AccessCheck::HasCredits(20));
        assert_eq!(AccessCheck::HasTrial(2), AccessCheck::HasTrial(2));
        assert_eq!(AccessCheck::NotAuthenticated, AccessCheck::NotAuthenticated);
        assert_eq!(AccessCheck::NeedsUpgrade, AccessCheck::NeedsUpgrade);
        assert_ne!(AccessCheck::HasCredits(10), AccessCheck::NeedsUpgrade);
    }

    #[test]
    fn purchase_urls_custom_config() {
        use super::super::CloudConfig;
        let client = CloudClient::with_config(CloudConfig {
            api_url: "http://localhost:3000".to_string(),
        });
        assert_eq!(client.subscription_url(), "http://localhost:3000/api/credits/purchase");
        assert_eq!(client.topup_url(), "http://localhost:3000/api/credits/purchase");
    }

    #[test]
    fn all_features_have_unique_keys() {
        let features = [Feature::Logo, Feature::Name, Feature::NorthStar, Feature::Scrape, Feature::Crawl, Feature::Vision, Feature::Search];
        let keys: Vec<&str> = features.iter().map(|f| f.key()).collect();
        let unique: std::collections::HashSet<&str> = keys.iter().copied().collect();
        assert_eq!(keys.len(), unique.len());
    }

    #[test]
    fn all_features_have_positive_costs() {
        let features = [Feature::Logo, Feature::Name, Feature::NorthStar, Feature::Scrape, Feature::Crawl, Feature::Vision, Feature::Search];
        for f in &features {
            assert!(f.cost() > 0, "{:?} should have positive cost", f);
        }
    }
}
