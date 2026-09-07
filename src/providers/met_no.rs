use crate::http::get_json;
use crate::types::{
    CurrentWeather, ForecastDay, HourPoint, MarineConditions, Result, WeatherError,
};

const LOCATIONFORECAST: &str = "https://api.met.no/weatherapi/locationforecast/2.0/compact";
const OCEANFORECAST: &str = "https://api.met.no/weatherapi/oceanforecast/2.0/compact";

fn details(entry: &serde_json::Value) -> Option<&serde_json::Value> {
    entry.get("data")?.get("instant")?.get("details")
}

/// MET Norway is the fallback source for hourly and daily forecasts.
pub fn fetch_hourly(lat: f64, lon: f64, hours: usize) -> Result<Vec<HourPoint>> {
    let json = get_json(&forecast_url(lat, lon))?;
    parse_hourly(&json, hours)
}

pub fn fetch_daily(lat: f64, lon: f64, days: usize) -> Result<Vec<ForecastDay>> {
    let json = get_json(&forecast_url(lat, lon))?;
    parse_daily(&json, days)
}

fn forecast_url(lat: f64, lon: f64) -> String {
    format!("{}?lat={:.4}&lon={:.4}", LOCATIONFORECAST, lat, lon)
}

/// Ocean forecast used as marine fallback when Open-Meteo Marine fails.
pub fn fetch_marine(lat: f64, lon: f64) -> Result<MarineConditions> {
    let url = format!("{}?lat={:.4}&lon={:.4}", OCEANFORECAST, lat, lon);
    let json = get_json(&url)?;
    parse_marine(&json)
}

fn timeseries(json: &serde_json::Value) -> Result<Vec<serde_json::Value>> {
    json.get("properties")
        .and_then(|p| p.get("timeseries"))
        .and_then(|v| v.as_array())
        .cloned()
        .ok_or_else(|| WeatherError::ParseError("met.no response missing timeseries".into()))
}

/// Parses the compact locationforecast into hourly points.
pub fn parse_hourly(json: &serde_json::Value, limit: usize) -> Result<Vec<HourPoint>> {
    let series = timeseries(json)?;
    let mut points = Vec::with_capacity(limit);

    for entry in series.iter().take(limit) {
        let time = entry
            .get("time")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let Some(details) = details(entry) else {
            continue;
        };

        let temperature_c = details
            .get("air_temperature")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let wind_kmh = details
            .get("wind_speed")
            .and_then(|v| v.as_f64())
            .map(|ms| ms * 3.6)
            .unwrap_or(0.0);
        let precipitation_mm = entry
            .pointer("/data/next_1_hours/details/precipitation_amount")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let probability = entry
            .pointer("/data/next_1_hours/details/probability_of_precipitation")
            .and_then(|v| v.as_f64())
            .map(|v| v as i32);

        points.push(HourPoint {
            time,
            temperature_c,
            precipitation_mm,
            precip_probability_pct: probability,
            wind_kmh,
            weather_code: None,
        });
    }

    Ok(points)
}

/// Aggregates the compact locationforecast into calendar days.
pub fn parse_daily(json: &serde_json::Value, limit: usize) -> Result<Vec<ForecastDay>> {
    use std::collections::BTreeMap;

    let series = timeseries(json)?;
    let mut order: Vec<String> = Vec::new();
    let mut buckets: BTreeMap<String, DayBucket> = BTreeMap::new();

    for entry in &series {
        let stamp = entry.get("time").and_then(|v| v.as_str()).unwrap_or_default();
        if stamp.len() < 10 {
            continue;
        }
        let date = stamp[..10].to_string();

        let Some(details) = details(entry) else {
            continue;
        };
        let temperature = details
            .get("air_temperature")
            .and_then(|v| v.as_f64());
        let wind_kmh = details
            .get("wind_speed")
            .and_then(|v| v.as_f64())
            .map(|ms| ms * 3.6);
        let precipitation = entry
            .pointer("/data/next_1_hours/details/precipitation_amount")
            .and_then(|v| v.as_f64());

        let bucket = buckets.entry(date.clone()).or_insert_with(|| {
            order.push(date.clone());
            DayBucket {
                date,
                ..DayBucket::default()
            }
        });

        if let Some(temp) = temperature {
            bucket.temp_max = bucket.temp_max.map(|v| v.max(temp)).or(Some(temp));
            bucket.temp_min = bucket.temp_min.map(|v| v.min(temp)).or(Some(temp));
        }
        if let Some(wind) = wind_kmh {
            bucket.wind_max = bucket.wind_max.map(|v| v.max(wind)).or(Some(wind));
        }
        bucket.precipitation_sum += precipitation.unwrap_or(0.0);
        if bucket.symbol.is_none() {
            bucket.symbol = entry
                .pointer("/data/next_1_hours/summary/symbol_code")
                .and_then(|v| v.as_str())
                .map(str::to_string);
        }
    }

    Ok(order
        .into_iter()
        .filter_map(|date| buckets.remove(&date))
        .take(limit)
        .map(DayBucket::into_forecast_day)
        .collect())
}

#[derive(Default)]
struct DayBucket {
    date: String,
    temp_max: Option<f64>,
    temp_min: Option<f64>,
    wind_max: Option<f64>,
    precipitation_sum: f64,
    symbol: Option<String>,
}

impl DayBucket {
    fn into_forecast_day(self) -> ForecastDay {
        ForecastDay {
            date: self.date,
            temp_max_c: self.temp_max.unwrap_or(0.0),
            temp_min_c: self.temp_min.unwrap_or(0.0),
            precipitation_mm: Some(self.precipitation_sum),
            precip_probability_pct: None,
            wind_max_kmh: self.wind_max.unwrap_or(0.0),
            weather_code: None,
            sunrise_utc: None,
            sunset_utc: None,
        }
    }
}

/// Parses an oceanforecast response into marine conditions.
pub fn parse_marine(json: &serde_json::Value) -> Result<MarineConditions> {
    let series = timeseries(json)?;
    let entry = series.first().ok_or_else(|| {
        WeatherError::ParseError("oceanforecast has no timeseries entries".into())
    })?;
    let Some(details) = details(entry) else {
        return Err(WeatherError::ParseError(
            "oceanforecast entry missing instant details".into(),
        ));
    };

    let field = |key: &str| -> Option<f64> {
        match details.get(key) {
            Some(serde_json::Value::Number(n)) => n.as_f64(),
            Some(other) => other.get("value").and_then(|v| v.as_f64()),
            None => None,
        }
    };

    Ok(MarineConditions {
        wave_height_m: field("sea_surface_wave_height"),
        wave_direction_deg: field("sea_surface_wave_from_direction").map(|v| v as i32),
        wave_period_s: field("sea_surface_wave_mean_period"),
        swell_height_m: field("sea_surface_swell_wave_height"),
        source: "MET Norway Ocean".to_string(),
    })
}

/// Fallback current conditions built from the newest forecast hour.
///
/// wttr.in stays the primary current fallback; this keeps a degraded answer
/// available when both Open-Meteo and wttr.in fail but MET Norway works.
pub fn fetch_current_degraded(lat: f64, lon: f64) -> Result<CurrentWeather> {
    let hours = fetch_hourly(lat, lon, 1)?;
    let point = hours.first().ok_or_else(|| {
        WeatherError::ParseError("met.no returned no usable hours".into())
    })?;

    Ok(CurrentWeather {
        temperature_c: point.temperature_c,
        feels_like_c: point.temperature_c,
        humidity_pct: 0,
        pressure_hpa: 1013.25,
        wind_kmh: point.wind_kmh,
        wind_direction_deg: 0,
        wind_gusts_kmh: None,
        precipitation_mm: point.precipitation_mm,
        cloud_cover_pct: None,
        visibility_m: None,
        uv_index: None,
        is_day: true,
        weather_code: None,
        condition: "Unknown".to_string(),
        source: "MET Norway".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample() -> serde_json::Value {
        json!({
            "properties": {
                "timeseries": [
                    {
                        "time": "2026-08-24T11:00:00Z",
                        "data": {
                            "instant": { "details": {
                                "air_temperature": 21.9,
                                "wind_speed": 3.0,
                                "wind_from_direction": 200,
                                "relative_humidity": 55,
                                "air_pressure_at_sea_level": 1015.2
                            }},
                            "next_1_hours": {
                                "summary": { "symbol_code": "partlycloudy_day" },
                                "details": {
                                    "precipitation_amount": 0.3,
                                    "probability_of_precipitation": 45
                                }
                            }
                        }
                    },
                    {
                        "time": "2026-08-25T00:00:00Z",
                        "data": {
                            "instant": { "details": {
                                "air_temperature": 14.1,
                                "wind_speed": 5.5
                            }},
                            "next_1_hours": {
                                "summary": { "symbol_code": "rain" },
                                "details": { "precipitation_amount": 2.0 }
                            }
                        }
                    },
                    {
                        "time": "2026-08-25T06:00:00Z",
                        "data": {
                            "instant": { "details": {
                                "air_temperature": 16.7,
                                "wind_speed": 8.3
                            }},
                            "next_1_hours": {
                                "summary": { "symbol_code": "rain" },
                                "details": { "precipitation_amount": 1.1 }
                            }
                        }
                    }
                ]
            }
        })
    }

    #[test]
    fn parses_hourly() {
        let points = parse_hourly(&sample(), 2).unwrap();
        assert_eq!(points.len(), 2);
        assert_eq!(points[0].temperature_c, 21.9);
        assert_eq!(points[0].wind_kmh, 10.8);
        assert_eq!(points[0].precipitation_mm, 0.3);
        assert_eq!(points[0].precip_probability_pct, Some(45));
        assert_eq!(points[0].weather_code, None);
    }

    #[test]
    fn aggregates_daily_by_date() {
        let days = parse_daily(&sample(), 7).unwrap();
        assert_eq!(days.len(), 2);

        assert_eq!(days[0].date, "2026-08-24");
        assert_eq!(days[0].temp_max_c, 21.9);
        assert_eq!(days[0].temp_min_c, 21.9);
        assert_eq!(days[0].precipitation_mm, Some(0.3));

        assert_eq!(days[1].date, "2026-08-25");
        assert_eq!(days[1].temp_max_c, 16.7);
        assert_eq!(days[1].temp_min_c, 14.1);
        assert!((days[1].precipitation_mm.unwrap() - 3.1).abs() < 1e-9);
        assert!((days[1].wind_max_kmh - 29.88).abs() < 1e-9);
    }

    #[test]
    fn rejects_broken_series() {
        assert!(parse_hourly(&json!({}), 2).is_err());
        assert!(parse_daily(&json!({"properties": {}}), 2).is_err());
    }

    #[test]
    fn parses_ocean() {
        let sample = json!({
            "properties": {
                "timeseries": [{
                    "time": "2026-08-24T12:00:00Z",
                    "data": { "instant": { "details": {
                        "sea_surface_wave_height": 1.2,
                        "sea_surface_wave_from_direction": 245,
                        "sea_surface_wave_mean_period": 6.5,
                        "sea_surface_swell_wave_height": 0.8
                    }}}
                }]
            }
        });

        let marine = parse_marine(&sample).unwrap();
        assert_eq!(marine.wave_height_m, Some(1.2));
        assert_eq!(marine.wave_direction_deg, Some(245));
        assert_eq!(marine.source, "MET Norway Ocean");

        assert!(parse_marine(&json!({"properties": {"timeseries": []}})).is_err());
    }
}
