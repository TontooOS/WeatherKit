use crate::http::get_json;
use crate::types::{Place, Result, WeatherError};

/// Searches places with the keyless Open-Meteo geocoding API.
///
/// The Nominatim search is the fallback source.
pub fn search(query: &str, limit: usize) -> Result<Vec<Place>> {
    match open_meteo_search(query, limit) {
        Ok(places) if !places.is_empty() => Ok(places),
        _ => nominatim_search(query, limit),
    }
}

fn encoded(query: &str) -> String {
    let mut encoded = String::with_capacity(query.len());
    for byte in query.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                encoded.push(byte as char)
            }
            other => encoded.push_str(&format!("%{:02X}", other)),
        }
    }
    encoded
}

pub fn open_meteo_search(query: &str, limit: usize) -> Result<Vec<Place>> {
    let url = format!(
        "https://geocoding-api.open-meteo.com/v1/search?name={}&count={}&language=en&format=json",
        encoded(query),
        limit.clamp(1, 10)
    );
    let json = get_json(&url)?;
    parse_open_meteo(&json)
}

pub fn parse_open_meteo(json: &serde_json::Value) -> Result<Vec<Place>> {
    let empty = Vec::new();
    let results = json
        .get("results")
        .and_then(|v| v.as_array())
        .unwrap_or(&empty);

    Ok(results
        .iter()
        .filter_map(|entry| {
            Some(Place {
                name: entry.get("name")?.as_str()?.to_string(),
                country: entry.get("country").and_then(|v| v.as_str()).map(str::to_string),
                region: entry.get("admin1").and_then(|v| v.as_str()).map(str::to_string),
                latitude: entry.get("latitude")?.as_f64()?,
                longitude: entry.get("longitude")?.as_f64()?,
            })
        })
        .collect())
}

pub fn nominatim_search(query: &str, limit: usize) -> Result<Vec<Place>> {
    use crate::http::USER_AGENT;
    use crate::http::map_reqwest_error;
    use std::time::Duration;

    let client = reqwest::blocking::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| WeatherError::NetworkError(e.to_string()))?;

    let url = format!(
        "https://nominatim.openstreetmap.org/search?q={}&format=json&limit={}",
        encoded(query),
        limit.clamp(1, 10)
    );

    let response = client.get(&url).send().map_err(map_reqwest_error)?;
    if !response.status().is_success() {
        return Err(WeatherError::ProviderFailed(format!(
            "Nominatim returned status {}",
            response.status()
        )));
    }

    let json: serde_json::Value = response
        .json()
        .map_err(|e| WeatherError::ParseError(e.to_string()))?;
    parse_nominatim(&json)
}

pub fn parse_nominatim(json: &serde_json::Value) -> Result<Vec<Place>> {
    let array = json
        .as_array()
        .ok_or_else(|| WeatherError::ParseError("nominatim response not a list".into()))?;

    Ok(array
        .iter()
        .filter_map(|entry| {
            let display = entry.get("display_name")?.as_str()?;
            let latitude = entry.get("lat")?.as_str()?.parse::<f64>().ok()?;
            let longitude = entry.get("lon")?.as_str()?.parse::<f64>().ok()?;

            let mut parts = display.splitn(2, ',');
            let name = parts.next().unwrap_or_default().trim().to_string();
            let rest = parts.next().unwrap_or_default();

            Some(Place {
                name,
                country: None,
                region: rest
                    .rsplit(',')
                    .nth(1)
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string),
                latitude,
                longitude,
            })
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn encodes_queries() {
        assert_eq!(encoded("New York"), "New%20York");
        assert_eq!(encoded("München"), "M%C3%BCnchen");
        assert_eq!(encoded("a-b_c.d~e"), "a-b_c.d~e");
    }

    #[test]
    fn parses_open_meteo_results() {
        let sample = json!({
            "results": [
                {"name": "Berlin", "country": "Germany", "admin1": "Land Berlin",
                 "latitude": 52.52437, "longitude": 13.41053},
                {"name": "Berlin", "country": "United States",
                 "latitude": 44.46867, "longitude": -71.18508}
            ]
        });

        let places = parse_open_meteo(&sample).unwrap();
        assert_eq!(places.len(), 2);
        assert_eq!(places[0].name, "Berlin");
        assert_eq!(places[0].country.as_deref(), Some("Germany"));
        assert_eq!(places[0].region.as_deref(), Some("Land Berlin"));
        assert_eq!(places[1].latitude, 44.46867);

        assert!(parse_open_meteo(&json!({})).unwrap().is_empty());
    }

    #[test]
    fn parses_nominatim_results() {
        let sample = json!([
            {
                "display_name": "Berlin, Land Berlin, 10117, Deutschland",
                "lat": "52.5170365",
                "lon": "13.3888599"
            }
        ]);

        let places = parse_nominatim(&sample).unwrap();
        assert_eq!(places.len(), 1);
        assert_eq!(places[0].name, "Berlin");
        assert_eq!(places[0].region.as_deref(), Some("10117"));
        assert!((places[0].latitude - 52.5170365).abs() < f64::EPSILON);

        assert!(parse_nominatim(&json!({"error": "x"})).is_err());
        assert!(parse_nominatim(&json!([]))
            .map(|places| places.is_empty())
            .unwrap_or(false));
    }
}
