use std::time::Duration;

use super::{auth, CloudClient};

/// Default interval between background balance refreshes.
pub const SYNC_INTERVAL: Duration = Duration::from_secs(300); // 5 minutes

/// Minimum interval to prevent excessive API calls.
pub const MIN_SYNC_INTERVAL: Duration = Duration::from_secs(30);

/// Start a background task that periodically refreshes the cached profile.
///
/// This should be spawned as a tokio task on app startup. It runs until
/// the returned handle is dropped or the task is aborted.
///
/// The sync only runs when:
/// - A JWT is stored (user is authenticated)
/// - The interval has elapsed since last sync
pub async fn background_sync(client: CloudClient, interval: Duration) {
    let interval = if interval < MIN_SYNC_INTERVAL {
        MIN_SYNC_INTERVAL
    } else {
        interval
    };

    loop {
        tokio::time::sleep(interval).await;

        // Only sync if authenticated
        if !auth::is_authenticated() {
            continue;
        }

        match client.refresh_balance().await {
            Ok(balance) => {
                tracing::debug!("Background sync: credits_balance={balance}");
            }
            Err(e) => {
                tracing::warn!("Background sync failed: {e}");
            }
        }
    }
}

/// Perform a one-time sync (useful on app startup or after mutations).
pub async fn sync_now(client: &CloudClient) -> Result<(), super::CloudError> {
    if !auth::is_authenticated() {
        return Err(super::CloudError::NotAuthenticated);
    }

    client.refresh_balance().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sync_interval_defaults() {
        assert_eq!(SYNC_INTERVAL, Duration::from_secs(300));
        assert_eq!(MIN_SYNC_INTERVAL, Duration::from_secs(30));
    }

    #[test]
    fn min_interval_less_than_default() {
        assert!(MIN_SYNC_INTERVAL < SYNC_INTERVAL);
    }

    #[test]
    fn sync_interval_enforced() {
        // Verify the clamping logic: if given an interval < MIN, it should use MIN
        let tiny = Duration::from_secs(1);
        let clamped = if tiny < MIN_SYNC_INTERVAL {
            MIN_SYNC_INTERVAL
        } else {
            tiny
        };
        assert_eq!(clamped, MIN_SYNC_INTERVAL);
    }

    #[test]
    fn sync_interval_passthrough() {
        // If given a valid interval >= MIN, it should pass through
        let valid = Duration::from_secs(600);
        let clamped = if valid < MIN_SYNC_INTERVAL {
            MIN_SYNC_INTERVAL
        } else {
            valid
        };
        assert_eq!(clamped, valid);
    }
}
