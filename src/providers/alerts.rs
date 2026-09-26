use crate::http::get_json;
use crate::types::{
    AlertSeverity, CurrentWeather, ForecastDay, HourPoint, Result, WeatherAlert, WeatherError,
};

const METALERTS: &str = "https://api.met.no/weatherapi/metalerts/2.0/current.json";

fn alerts_url(lat: f64, lon: f64) -> String {
    format!("{}?lat={:.4}&lon={:.4}", METALERTS, lat, lon)
}

fn str_field(value: &serde_json::Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(text) = value.get(*key).and_then(|v| v.as_str()) {
            if !text.is_empty() {
                return Some(text.to_string());
            }
        }
    }
    None
}

/// Parses a MET Norway MetAlerts GeoJSON response.
///
/// Tolerant by design: unknown shapes yield an empty list instead of an error
/// because alerts are advisory data, and an empty list outside Europe (where
/// MetAlerts has no coverage) is a valid answer.
pub fn parse_metalerts(json: &serde_json::Value) -> Vec<WeatherAlert> {
    let empty = Vec::new();
    let features = json
        .get("features")
        .and_then(|v| v.as_array())
        .unwrap_or(&empty);

    features
        .iter()
        .filter_map(|feature| {
            let props = feature.get("properties")?;
            let headline = str_field(props, &["event", "headline", "title"])?;
            Some(WeatherAlert {
                description: str_field(
                    props,
                    &["description", "instruction", "consequences"],
                )
                .unwrap_or_default(),
                severity: AlertSeverity::from_str(
                    str_field(props, &["severity", "riskLevel", "level"])
                        .unwrap_or_default()
                        .as_str(),
                ),
                kind: str_field(props, &["event", "awareness_type"])
                    .unwrap_or_else(|| "Weather".to_string()),
                effective: str_field(props, &["effective", "onset"]),
                expires: str_field(props, &["expires", "expiry"]),
                headline,
                source: "MET Norway MetAlerts".to_string(),
            })
        })
        .collect()
}

/// Fetches official alerts from MET Norway (keyless, Europe coverage).
///
/// Returns `Err` when the network fails so callers can fall back to locally
/// synthesized alerts; an empty list is a valid answer outside Europe.
pub fn fetch_metalerts(lat: f64, lon: f64) -> Result<Vec<WeatherAlert>> {
    let json = get_json(&alerts_url(lat, lon)).map_err(|e| match e {
        WeatherError::ProviderFailed(msg) if msg.contains("404") => {
            WeatherError::ProviderFailed("no metalerts coverage here".to_string())
        }
        other => other,
    })?;
    Ok(parse_metalerts(&json))
}

fn alert(
    kind: &str,
    severity: AlertSeverity,
    headline: &str,
    description: String,
) -> WeatherAlert {
    WeatherAlert {
        headline: headline.to_string(),
        description,
        severity,
        kind: kind.to_string(),
        source: "WeatherKit Synthesis (keyless)".to_string(),
        effective: None,
        expires: None,
    }
}

/// Builds advisory alerts from local forecast data (fully offline, global).
///
/// Thresholds are conservative so the list stays empty in normal weather.
/// Every condition is derived from keyless sources only; no API key involved.
pub fn synthesize(
    current: &CurrentWeather,
    hourly: &[HourPoint],
    daily: &[ForecastDay],
) -> Vec<WeatherAlert> {
    let mut alerts = Vec::new();

    let max_wind = hourly
        .iter()
        .map(|h| h.wind_kmh)
        .fold(current.wind_kmh, f64::max);
    if max_wind >= 100.0 {
        alerts.push(alert(
            "Storm",
            AlertSeverity::Extreme,
            "Extreme storm",
            format!("Wind gusts up to {:.0} km/h expected.", max_wind),
        ));
    } else if max_wind >= 75.0 {
        alerts.push(alert(
            "Storm",
            AlertSeverity::Severe,
            "Storm warning",
            format!("Wind up to {:.0} km/h expected.", max_wind),
        ));
    } else if max_wind >= 60.0 {
        alerts.push(alert(
            "Wind",
            AlertSeverity::Moderate,
            "Strong wind",
            format!("Wind up to {:.0} km/h expected.", max_wind),
        ));
    }

    let max_hour_rain = hourly
        .iter()
        .map(|h| h.precipitation_mm)
        .fold(0.0_f64, f64::max);
    let max_day_rain = daily
        .iter()
        .filter_map(|d| d.precipitation_mm)
        .fold(0.0_f64, f64::max);
    if max_hour_rain >= 15.0 || max_day_rain >= 50.0 {
        alerts.push(alert(
            "HeavyRain",
            AlertSeverity::Severe,
            "Heavy rain",
            format!(
                "Up to {:.1} mm/h, daily total {:.1} mm.",
                max_hour_rain, max_day_rain
            ),
        ));
    } else if max_hour_rain >= 7.5 || max_day_rain >= 25.0 {
        alerts.push(alert(
            "HeavyRain",
            AlertSeverity::Moderate,
            "Rain warning",
            format!(
                "Up to {:.1} mm/h, daily total {:.1} mm.",
                max_hour_rain, max_day_rain
            ),
        ));
    }

    let thunderstorm = hourly.iter().any(|h| matches!(h.weather_code, Some(95) | Some(96) | Some(99)))
        || daily.iter().any(|d| matches!(d.weather_code, Some(95) | Some(96) | Some(99)));
    if thunderstorm {
        alerts.push(alert(
            "Thunderstorm",
            AlertSeverity::Moderate,
            "Thunderstorm expected",
            "Thunderstorm with possible hail in the forecast.".to_string(),
        ));
    }

    let max_temp = daily
        .iter()
        .map(|d| d.temp_max_c)
        .fold(current.temperature_c, f64::max);
    if max_temp >= 38.0 {
        alerts.push(alert(
            "Heat",
            AlertSeverity::Severe,
            "Extreme heat",
            format!("Up to {:.0} C expected.", max_temp),
        ));
    } else if max_temp >= 32.0 {
        alerts.push(alert(
            "Heat",
            AlertSeverity::Moderate,
            "Heat warning",
            format!("Up to {:.0} C expected.", max_temp),
        ));
    }

    let min_temp = daily
        .iter()
        .map(|d| d.temp_min_c)
        .fold(current.temperature_c, f64::min);
    if min_temp <= -15.0 {
        alerts.push(alert(
            "Frost",
            AlertSeverity::Severe,
            "Extreme frost",
            format!("Down to {:.0} C expected.", min_temp),
        ));
    } else if min_temp <= -5.0 {
        alerts.push(alert(
            "Frost",
            AlertSeverity::Moderate,
            "Frost warning",
            format!("Down to {:.0} C expected.", min_temp),
        ));
    }

    if let Some(uv) = current.uv_index {
        if uv >= 9.0 {
            alerts.push(alert(
                "Ultraviolet",
                AlertSeverity::Severe,
                "Extreme UV index",
                format!("Current UV index {:.0}. Avoid midday sun.", uv),
            ));
        } else if uv >= 6.0 {
            alerts.push(alert(
                "Ultraviolet",
                AlertSeverity::Minor,
                "High UV index",
                format!("Current UV index {:.0}. Use sun protection.", uv),
            ));
        }
    }

    if let Some(visibility) = current.visibility_m {
        if visibility < 200.0 {
            alerts.push(alert(
                "Fog",
                AlertSeverity::Moderate,
                "Dense fog",
                format!("Visibility {:.0} m.", visibility),
            ));
        } else if visibility < 1000.0 {
            alerts.push(alert(
                "Fog",
                AlertSeverity::Minor,
                "Fog",
                format!("Visibility {:.0} m.", visibility),
            ));
        }
    }

    alerts
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn calm_current() -> CurrentWeather {
        CurrentWeather {
            temperature_c: 20.0,
            uv_index: Some(3.0),
            visibility_m: Some(20000.0),
            wind_kmh: 10.0,
            ..CurrentWeather::default()
        }
    }

    #[test]
    fn parses_metalerts_features() {
        let sample = json!({
            "features": [
                {"properties": {
                    "event": "Strong wind",
                    "severity": "moderate",
                    "description": "Secure loose objects",
                    "effective": "2026-08-24T10:00:00Z",
                    "expires": "2026-08-24T20:00:00Z"
                }},
                {"properties": {"nope": true}},
                {}
            ]
        });

        let alerts = parse_metalerts(&sample);
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].headline, "Strong wind");
        assert_eq!(alerts[0].severity, AlertSeverity::Moderate);
        assert!(alerts[0].should_notify());
        assert_eq!(alerts[0].source, "MET Norway MetAlerts");
    }

    #[test]
    fn empty_outside_coverage() {
        assert!(parse_metalerts(&json!({})).is_empty());
        assert!(parse_metalerts(&json!({"features": []})).is_empty());
    }

    #[test]
    fn calm_weather_synthesizes_nothing() {
        let alerts = synthesize(&calm_current(), &[], &[]);
        assert!(alerts.is_empty());
    }

    #[test]
    fn storm_heat_and_thunder_synthesized() {
        let current = CurrentWeather {
            temperature_c: 39.0,
            wind_kmh: 110.0,
            uv_index: Some(10.0),
            ..CurrentWeather::default()
        };
        let hourly = vec![HourPoint {
            time: "2026-08-24T12:00".to_string(),
            temperature_c: 39.0,
            precipitation_mm: 20.0,
            precip_probability_pct: Some(90),
            wind_kmh: 110.0,
            weather_code: Some(95),
        }];
        let daily = vec![ForecastDay {
            date: "2026-08-24".to_string(),
            temp_max_c: 39.0,
            temp_min_c: 22.0,
            precipitation_mm: Some(60.0),
            precip_probability_pct: Some(90),
            wind_max_kmh: 110.0,
            weather_code: Some(95),
            sunrise_utc: None,
            sunset_utc: None,
        }];

        let alerts = synthesize(&current, &hourly, &daily);
        let kinds: Vec<&str> = alerts.iter().map(|a| a.kind.as_str()).collect();
        assert!(kinds.contains(&"Storm"));
        assert!(kinds.contains(&"HeavyRain"));
        assert!(kinds.contains(&"Thunderstorm"));
        assert!(kinds.contains(&"Heat"));
        assert!(alerts.iter().any(|a| a.should_notify()));
    }
}
