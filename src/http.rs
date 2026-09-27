use crate::types::{Result, WeatherError};
use networkkit::http::{HttpClient, HttpResponse};
use std::time::Duration;

pub const USER_AGENT: &str = "TontooOS-WeatherKit/26.1 (TontooOS weather framework)";
const TIMEOUT_SECS: u64 = 10;

fn client() -> HttpClient {
    HttpClient::with_user_agent(USER_AGENT).timeout(Duration::from_secs(TIMEOUT_SECS))
}

pub fn get_json(url: &str) -> Result<foundation::serialization::JsonValue> {
    let response = client().get(url).send().map_err(map_network_error)?;

    if !response.is_success() {
        if response.status == 429 {
            return Err(WeatherError::ProviderFailed(format!(
                "rate limited by {}",
                url.split('/').nth(2).unwrap_or("server")
            )));
        }
        return Err(WeatherError::ProviderFailed(format!(
            "HTTP {} from {}",
            response.status,
            url.split('/').nth(2).unwrap_or("server")
        )));
    }

    response_json(response)
}

/// Parses a response body as JSON, mapping transport leftovers to network
/// errors and bad payloads to parse errors.
pub fn response_json(
    response: HttpResponse,
) -> Result<foundation::serialization::JsonValue> {
    let text = response.text().map_err(|e| match e {
        networkkit::types::NetworkError::ParseError(msg) => WeatherError::ParseError(msg),
        other => WeatherError::NetworkError(other.to_string()),
    })?;
    foundation::serialization::JsonValue::parse(&text)
        .map_err(|e| WeatherError::ParseError(e.to_string()))
}

pub fn map_network_error(err: networkkit::types::NetworkError) -> WeatherError {
    use networkkit::types::NetworkError as NetErr;
    match err {
        NetErr::Timeout => WeatherError::Timeout,
        NetErr::InvalidUrl(_)
        | NetErr::NotAvailable
        | NetErr::PermissionDenied => WeatherError::NotAvailable,
        NetErr::ParseError(msg) => WeatherError::ParseError(msg),
        NetErr::HttpError(msg) | NetErr::CommandFailed(msg) | NetErr::IoError(msg) => {
            WeatherError::NetworkError(msg)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_errors() {
        // Invalid URLs never touch the network.
        let mapped = map_network_error(
            networkkit::http::HttpRequest::get("not-a-url")
                .send()
                .unwrap_err(),
        );
        assert!(matches!(mapped, WeatherError::NotAvailable));

        let mapped = map_network_error(networkkit::types::NetworkError::Timeout);
        assert!(matches!(mapped, WeatherError::Timeout));

        let mapped = map_network_error(networkkit::types::NetworkError::HttpError(
            "connection refused".into(),
        ));
        assert!(matches!(mapped, WeatherError::NetworkError(_)));
    }
}
