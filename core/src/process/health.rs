use anyhow::Result;
use std::time::Duration;

/// Check if a service is healthy by hitting its health URL.
pub async fn check_health(url: &str, timeout: Duration) -> Result<bool> {
    let client = reqwest::Client::builder().timeout(timeout).build()?;

    match client.get(url).send().await {
        Ok(resp) => Ok(resp.status().is_success()),
        Err(_) => Ok(false),
    }
}
