use crate::http::get_json;
use crate::types::{CurrentWeather, Result, WeatherError};

/// wttr.in is the fallback source for current conditions.
pub fn fetch_current(lat: f64, lon: f64) -> Result<CurrentWeather> {
    let url = format!("https://wttr.in/{:.4},{:.4}?format=j1", lat, lon);
    let json = get_json(&url)?;
    parse_current(&json)
}

/// Parses a wttr.in `format=j1` response.
pub fn parse_current(json: &serde_json::Value) -> Result<CurrentWeather> {
    let condition = json
        .get("current_condition")
        .and_then(|v| v.as_array())
        .and_then(|v| v.first())
        .ok_or_else(|| WeatherError::ParseError("wttr response missing current_condition".into()))?;

    let f64_field = |key: &str| -> Option<f64> {
        condition.get(key)?.as_str()?.parse::<f64>().ok()
    };

    let temperature_c = f64_field("temp_C")
        .ok_or_else(|| WeatherError::ParseError("wttr missing temp_C".into()))?;

    Ok(CurrentWeather {
        temperature_c,
        feels_like_c: f64_field("FeelsLikeC").unwrap_or(temperature_c),
        humidity_pct: f64_field("humidity").map(|v| v as i32).unwrap_or(0),
        pressure_hpa: f64_field("pressure").unwrap_or(1013.25),
        wind_kmh: f64_field("windspeedKmph").unwrap_or(0.0),
        wind_direction_deg: f64_field("winddirDegree").map(|v| v as i32).unwrap_or(0),
        wind_gusts_kmh: None,
        precipitation_mm: f64_field("precipMM").unwrap_or(0.0),
        cloud_cover_pct: f64_field("cloudcover").map(|v| v as i32),
        visibility_m: f64_field("visibility").map(|km| km * 1000.0),
        uv_index: f64_field("uvIndex"),
        is_day: true,
        weather_code: None,
        condition: condition
            .get("weatherDesc")
            .and_then(|v| v.as_array())
            .and_then(|v| v.first())
            .and_then(|v| v.get("value"))
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown")
            .to_string(),
        source: "wttr.in".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_j1_current() {
        let sample = json!({
            "current_condition": [{
                "temp_C": "24",
                "FeelsLikeC": "26",
                "humidity": "48",
                "pressure": "1016",
                "precipMM": "0.1",
                "uvIndex": "5",
                "visibility": "10",
                "cloudcover": "60",
                "windspeedKmph": "12",
                "winddirDegree": "212",
                "weatherDesc": [{"value": "Partly cloudy"}]
            }],
            "weather": []
        });

        let weather = parse_current(&sample).unwrap();
        assert_eq!(weather.temperature_c, 24.0);
        assert_eq!(weather.feels_like_c, 26.0);
        assert_eq!(weather.humidity_pct, 48);
        assert_eq!(weather.pressure_hpa, 1016.0);
        assert_eq!(weather.wind_kmh, 12.0);
        assert_eq!(weather.wind_direction_deg, 212);
        assert_eq!(weather.precipitation_mm, 0.1);
        assert_eq!(weather.visibility_m, Some(10000.0));
        assert_eq!(weather.uv_index, Some(5.0));
        assert_eq!(weather.condition, "Partly cloudy");
        assert_eq!(weather.source, "wttr.in");
    }

    #[test]
    fn rejects_broken_j1() {
        assert!(parse_current(&json!({})).is_err());
        assert!(parse_current(&json!({"current_condition": [{}]})).is_err());
    }
}
