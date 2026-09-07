use crate::types::{Result, WeatherError};
use std::time::Duration;

pub const USER_AGENT: &str = "TontooOS-WeatherKit/26.1 (TontooOS weather framework)";
const TIMEOUT_SECS: u64 = 10;

fn client() -> Result<reqwest::blocking::Client> {
    reqwest::blocking::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(TIMEOUT_SECS))
        .build()
        .map_err(|e| WeatherError::NetworkError(e.to_string()))
}

pub fn get_json(url: &str) -> Result<serde_json::Value> {
    let response = client()?
        .get(url)
        .send()
        .map_err(map_reqwest_error)?;

    let status = response.status();
    if !status.is_success() {
        if status.as_u16() == 429 {
            return Err(WeatherError::ProviderFailed(format!(
                "rate limited by {}",
                url.split('/').nth(2).unwrap_or("server")
            )));
        }
        return Err(WeatherError::ProviderFailed(format!(
            "HTTP {} from {}",
            status,
            url.split('/').nth(2).unwrap_or("server")
        )));
    }

    response
        .json::<serde_json::Value>()
        .map_err(|e| WeatherError::ParseError(e.to_string()))
}

pub fn map_reqwest_error(err: reqwest::Error) -> WeatherError {
    if err.is_timeout() {
        WeatherError::Timeout
    } else if err.is_connect() || err.is_request() {
        WeatherError::NotAvailable
    } else {
        WeatherError::NetworkError(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_errors() {
        let raw = reqwest::blocking::get("http://127.0.0.1:1/x").unwrap_err();
        let mapped = map_reqwest_error(raw);
        assert!(matches!(
            mapped,
            WeatherError::NotAvailable | WeatherError::Timeout | WeatherError::NetworkError(_)
        ));
    }
}
