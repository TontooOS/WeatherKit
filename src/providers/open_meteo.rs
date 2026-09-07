use crate::http::get_json;
use crate::types::{
    weather_code_description, CurrentWeather, ForecastDay, HistoricalDay, HourPoint,
    MarineConditions, Result, WeatherError,
};

pub const FORECAST_API: &str = "https://api.open-meteo.com/v1/forecast";
pub const ARCHIVE_API: &str = "https://archive-api.open-meteo.com/v1/archive";
pub const MARINE_API: &str = "https://marine-api.open-meteo.com/v1/marine";

const CURRENT_FIELDS: &str = "temperature_2m,apparent_temperature,relative_humidity_2m,\
surface_pressure,pressure_msl,precipitation,weather_code,cloud_cover,wind_speed_10m,\
wind_direction_10m,wind_gusts_10m,is_day,uv_index";

fn num(value: &serde_json::Value, key: &str) -> Option<f64> {
    value.get(key)?.as_f64()
}

fn int(value: &serde_json::Value, key: &str) -> Option<i32> {
    value.get(key)?.as_i64().map(|v| v as i32)
}

/// Builds the forecast URL for current conditions.
pub fn current_url(lat: f64, lon: f64) -> String {
    format!(
        "{}?latitude={:.5}&longitude={:.5}&current={}&wind_speed_unit=kmh",
        FORECAST_API, lat, lon, CURRENT_FIELDS
    )
}

/// Parses an Open-Meteo forecast response containing a `current` object.
pub fn parse_current(json: &serde_json::Value, source: &str) -> Result<CurrentWeather> {
    let current = json.get("current").ok_or_else(|| {
        WeatherError::ParseError("open-meteo response missing current".into())
    })?;

    let temperature_c = num(current, "temperature_2m")
        .ok_or_else(|| WeatherError::ParseError("missing temperature_2m".into()))?;
    let code = int(current, "weather_code");

    Ok(CurrentWeather {
        temperature_c,
        feels_like_c: num(current, "apparent_temperature").unwrap_or(temperature_c),
        humidity_pct: num(current, "relative_humidity_2m").map(|v| v as i32).unwrap_or(0),
        pressure_hpa: num(current, "pressure_msl")
            .or_else(|| num(current, "surface_pressure"))
            .unwrap_or(1013.25),
        wind_kmh: num(current, "wind_speed_10m").unwrap_or(0.0),
        wind_direction_deg: num(current, "wind_direction_10m").map(|v| v as i32).unwrap_or(0),
        wind_gusts_kmh: num(current, "wind_gusts_10m"),
        precipitation_mm: num(current, "precipitation").unwrap_or(0.0),
        cloud_cover_pct: num(current, "cloud_cover").map(|v| v as i32),
        visibility_m: None,
        uv_index: num(current, "uv_index"),
        is_day: int(current, "is_day").map(|v| v == 1).unwrap_or(true),
        weather_code: code,
        condition: code
            .map(weather_code_description)
            .unwrap_or("Unknown")
            .to_string(),
        source: source.to_string(),
    })
}

/// Fetches current conditions from the primary Open-Meteo endpoint.
pub fn fetch_current(lat: f64, lon: f64) -> Result<CurrentWeather> {
    let json = get_json(&current_url(lat, lon))?;
    parse_current(&json, "Open-Meteo")
}

/// Builds the hourly/daily forecast URL.
pub fn forecast_url(lat: f64, lon: f64, days: u8) -> String {
    format!(
        "{}?latitude={:.5}&longitude={:.5}\
&hourly=temperature_2m,precipitation,precipitation_probability,wind_speed_10m,weather_code\
&daily=temperature_2m_max,temperature_2m_min,precipitation_sum,precipitation_probability_max,\
wind_speed_10m_max,weather_code,sunrise,sunset\
&forecast_days={}&wind_speed_unit=kmh&timezone=auto",
        FORECAST_API, lat, lon, days
    )
}

/// Parses hourly arrays into [`HourPoint`] values.
pub fn parse_hourly(json: &serde_json::Value, limit: usize) -> Result<Vec<HourPoint>> {
    let hourly = json
        .get("hourly")
        .ok_or_else(|| WeatherError::ParseError("missing hourly block".into()))?;
    let empty = Vec::new();
    let times = hourly
        .get("time")
        .and_then(|v| v.as_array())
        .unwrap_or(&empty);
    let temps = hourly
        .get("temperature_2m")
        .and_then(|v| v.as_array())
        .unwrap_or(&empty);
    let precip = hourly
        .get("precipitation")
        .and_then(|v| v.as_array())
        .unwrap_or(&empty);
    let prob = hourly
        .get("precipitation_probability")
        .and_then(|v| v.as_array())
        .unwrap_or(&empty);
    let wind = hourly
        .get("wind_speed_10m")
        .and_then(|v| v.as_array())
        .unwrap_or(&empty);
    let codes = hourly
        .get("weather_code")
        .and_then(|v| v.as_array())
        .unwrap_or(&empty);

    let mut points = Vec::with_capacity(limit.min(times.len()));
    for index in 0..times.len() {
        if points.len() >= limit {
            break;
        }
        points.push(HourPoint {
            time: times[index].as_str().unwrap_or_default().to_string(),
            temperature_c: temps.get(index).and_then(|v| v.as_f64()).unwrap_or(0.0),
            precipitation_mm: precip.get(index).and_then(|v| v.as_f64()).unwrap_or(0.0),
            precip_probability_pct: prob.get(index).and_then(|v| v.as_i64()).map(|v| v as i32),
            wind_kmh: wind.get(index).and_then(|v| v.as_f64()).unwrap_or(0.0),
            weather_code: codes.get(index).and_then(|v| v.as_i64()).map(|v| v as i32),
        });
    }

    Ok(points)
}

pub fn fetch_hourly(lat: f64, lon: f64, hours: usize) -> Result<Vec<HourPoint>> {
    let json = get_json(&forecast_url(lat, lon, 3))?;
    parse_hourly(&json, hours)
}

/// Parses daily arrays into [`ForecastDay`] values.
pub fn parse_daily(json: &serde_json::Value, limit: usize) -> Result<Vec<ForecastDay>> {
    let daily = json
        .get("daily")
        .ok_or_else(|| WeatherError::ParseError("missing daily block".into()))?;
    let empty = Vec::new();
    let dates = daily
        .get("time")
        .and_then(|v| v.as_array())
        .unwrap_or(&empty);
    let tmax = daily
        .get("temperature_2m_max")
        .and_then(|v| v.as_array())
        .unwrap_or(&empty);
    let tmin = daily
        .get("temperature_2m_min")
        .and_then(|v| v.as_array())
        .unwrap_or(&empty);
    let psum = daily
        .get("precipitation_sum")
        .and_then(|v| v.as_array())
        .unwrap_or(&empty);
    let pprob = daily
        .get("precipitation_probability_max")
        .and_then(|v| v.as_array())
        .unwrap_or(&empty);
    let wmax = daily
        .get("wind_speed_10m_max")
        .and_then(|v| v.as_array())
        .unwrap_or(&empty);
    let codes = daily
        .get("weather_code")
        .and_then(|v| v.as_array())
        .unwrap_or(&empty);
    let sunrise = daily
        .get("sunrise")
        .and_then(|v| v.as_array())
        .unwrap_or(&empty);
    let sunset = daily
        .get("sunset")
        .and_then(|v| v.as_array())
        .unwrap_or(&empty);

    let mut days = Vec::with_capacity(limit.min(dates.len()));
    for index in 0..dates.len() {
        if days.len() >= limit {
            break;
        }
        days.push(ForecastDay {
            date: dates[index].as_str().unwrap_or_default().to_string(),
            temp_max_c: tmax.get(index).and_then(|v| v.as_f64()).unwrap_or(0.0),
            temp_min_c: tmin.get(index).and_then(|v| v.as_f64()).unwrap_or(0.0),
            precipitation_mm: psum.get(index).and_then(|v| v.as_f64()),
            precip_probability_pct: pprob.get(index).and_then(|v| v.as_i64()).map(|v| v as i32),
            wind_max_kmh: wmax.get(index).and_then(|v| v.as_f64()).unwrap_or(0.0),
            weather_code: codes.get(index).and_then(|v| v.as_i64()).map(|v| v as i32),
            sunrise_utc: sunrise.get(index).and_then(|v| v.as_str()).map(str::to_string),
            sunset_utc: sunset.get(index).and_then(|v| v.as_str()).map(str::to_string),
        });
    }

    Ok(days)
}

pub fn fetch_daily(lat: f64, lon: f64, days: u8) -> Result<Vec<ForecastDay>> {
    let json = get_json(&forecast_url(lat, lon, days.max(1)))?;
    parse_daily(&json, days as usize)
}

/// Builds the archive URL for historical weather.
pub fn archive_url(lat: f64, lon: f64, start_date: &str, end_date: &str) -> String {
    format!(
        "{}?latitude={:.5}&longitude={:.5}\
&start_date={}&end_date={}\
&daily=temperature_2m_max,temperature_2m_min,precipitation_sum&timezone=auto",
        ARCHIVE_API, lat, lon, start_date, end_date
    )
}

/// Parses archive responses into [`HistoricalDay`] values.
pub fn parse_archive(json: &serde_json::Value) -> Result<Vec<HistoricalDay>> {
    let daily = json
        .get("daily")
        .ok_or_else(|| WeatherError::ParseError("missing daily block".into()))?;
    let empty = Vec::new();
    let dates = daily
        .get("time")
        .and_then(|v| v.as_array())
        .unwrap_or(&empty);
    let tmax = daily
        .get("temperature_2m_max")
        .and_then(|v| v.as_array())
        .unwrap_or(&empty);
    let tmin = daily
        .get("temperature_2m_min")
        .and_then(|v| v.as_array())
        .unwrap_or(&empty);
    let psum = daily
        .get("precipitation_sum")
        .and_then(|v| v.as_array())
        .unwrap_or(&empty);

    Ok((0..dates.len())
        .map(|index| HistoricalDay {
            date: dates[index].as_str().unwrap_or_default().to_string(),
            temp_max_c: tmax.get(index).and_then(|v| v.as_f64()),
            temp_min_c: tmin.get(index).and_then(|v| v.as_f64()),
            precipitation_mm: psum.get(index).and_then(|v| v.as_f64()),
        })
        .collect())
}

pub fn fetch_historical(
    lat: f64,
    lon: f64,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<HistoricalDay>> {
    let json = get_json(&archive_url(lat, lon, start_date, end_date))?;
    parse_archive(&json)
}

/// Marine conditions from the Open-Meteo marine API (ECMWF WAM).
pub fn fetch_marine(lat: f64, lon: f64) -> Result<MarineConditions> {
    let url = format!(
        "{}?latitude={:.5}&longitude={:.5}\
&current=wave_height,wave_direction,wave_period,swell_wave_height",
        MARINE_API, lat, lon
    );
    let json = get_json(&url)?;
    parse_marine(&json, "Open-Meteo Marine")
}

/// Parses a marine API response.
pub fn parse_marine(json: &serde_json::Value, source: &str) -> Result<MarineConditions> {
    let current = json
        .get("current")
        .ok_or_else(|| WeatherError::ParseError("marine response missing current".into()))?;

    Ok(MarineConditions {
        wave_height_m: num(current, "wave_height"),
        wave_direction_deg: num(current, "wave_direction").map(|v| v as i32),
        wave_period_s: num(current, "wave_period"),
        swell_height_m: num(current, "swell_wave_height"),
        source: source.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn builds_urls() {
        let url = current_url(52.52, 13.405);
        assert!(url.starts_with("https://api.open-meteo.com/v1/forecast?"));
        assert!(url.contains("latitude=52.52000"));
        assert!(url.contains("longitude=13.40500"));
        assert!(url.contains("temperature_2m"));

        let url = forecast_url(52.52, 13.405, 7);
        assert!(url.contains("forecast_days=7"));
        assert!(url.contains("timezone=auto"));

        let url = archive_url(1.0, 2.0, "2026-01-01", "2026-01-31");
        assert!(url.contains("start_date=2026-01-01"));
        assert!(url.contains("end_date=2026-01-31"));
        assert!(url.contains("archive-api.open-meteo.com"));
    }

    #[test]
    fn parses_current() {
        let sample = json!({
            "current": {
                "time": "2026-08-24T12:00",
                "temperature_2m": 24.3,
                "apparent_temperature": 25.9,
                "relative_humidity_2m": 48,
                "pressure_msl": 1016.4,
                "precipitation": 0.2,
                "weather_code": 2,
                "cloud_cover": 60,
                "wind_speed_10m": 11.8,
                "wind_direction_10m": 212,
                "wind_gusts_10m": 27.4,
                "is_day": 1,
                "uv_index": 4.85
            }
        });

        let weather = parse_current(&sample, "Open-Meteo").unwrap();
        assert_eq!(weather.temperature_c, 24.3);
        assert_eq!(weather.feels_like_c, 25.9);
        assert_eq!(weather.humidity_pct, 48);
        assert_eq!(weather.pressure_hpa, 1016.4);
        assert_eq!(weather.wind_kmh, 11.8);
        assert_eq!(weather.wind_direction_deg, 212);
        assert_eq!(weather.wind_gusts_kmh, Some(27.4));
        assert_eq!(weather.precipitation_mm, 0.2);
        assert_eq!(weather.cloud_cover_pct, Some(60));
        assert_eq!(weather.uv_index, Some(4.85));
        assert!(weather.is_day);
        assert_eq!(weather.weather_code, Some(2));
        assert_eq!(weather.condition, "Partly cloudy");
        assert_eq!(weather.source, "Open-Meteo");
    }

    #[test]
    fn rejects_broken_current() {
        assert!(parse_current(&json!({}), "x").is_err());
        assert!(parse_current(&json!({"current": {}}), "x").is_err());
    }

    #[test]
    fn parses_hourly_arrays() {
        let sample = json!({
            "hourly": {
                "time": ["2026-08-24T00:00", "2026-08-24T01:00", "2026-08-24T02:00"],
                "temperature_2m": [18.1, 17.4, 16.9],
                "precipitation": [0.0, 0.4, 1.1],
                "precipitation_probability": [5, 40, 80],
                "wind_speed_10m": [9.2, 10.1, 12.0],
                "weather_code": [1, 61, 63]
            }
        });

        let hours = parse_hourly(&sample, 2).unwrap();
        assert_eq!(hours.len(), 2);
        assert_eq!(hours[0].temperature_c, 18.1);
        assert_eq!(hours[1].precip_probability_pct, Some(40));
        assert_eq!(hours[1].weather_code, Some(61));

        assert!(parse_hourly(&json!({}), 2).is_err());
    }

    #[test]
    fn parses_daily_arrays() {
        let sample = json!({
            "daily": {
                "time": ["2026-08-24", "2026-08-25"],
                "temperature_2m_max": [28.4, 26.1],
                "temperature_2m_min": [15.2, 14.8],
                "precipitation_sum": [0.0, 4.2],
                "precipitation_probability_max": [10, 75],
                "wind_speed_10m_max": [18.3, 22.0],
                "weather_code": [1, 95],
                "sunrise": ["2026-08-24T04:43", "2026-08-25T04:44"],
                "sunset": ["2026-08-24T20:41", "2026-08-25T20:39"]
            }
        });

        let days = parse_daily(&sample, 5).unwrap();
        assert_eq!(days.len(), 2);
        assert_eq!(days[0].temp_max_c, 28.4);
        assert_eq!(days[1].weather_code, Some(95));
        assert_eq!(days[1].sunrise_utc.as_deref(), Some("2026-08-25T04:44"));

        assert!(parse_daily(&json!({}), 5).is_err());
    }

    #[test]
    fn parses_archive() {
        let sample = json!({
            "daily": {
                "time": ["2026-07-01", "2026-07-02"],
                "temperature_2m_max": [31.2, null],
                "temperature_2m_min": [19.4, 18.0],
                "precipitation_sum": [null, 12.5]
            }
        });

        let days = parse_archive(&sample).unwrap();
        assert_eq!(days.len(), 2);
        assert_eq!(days[0].temp_max_c, Some(31.2));
        assert_eq!(days[0].precipitation_mm, None);
        assert_eq!(days[1].temp_max_c, None);
        assert_eq!(days[1].precipitation_mm, Some(12.5));
    }

    #[test]
    fn parses_marine() {
        let sample = json!({
            "current": {
                "time": "2026-08-24T12:00",
                "wave_height": 1.34,
                "wave_direction": 245,
                "wave_period": 6.8,
                "swell_wave_height": 0.92
            }
        });

        let marine = parse_marine(&sample, "Open-Meteo Marine").unwrap();
        assert_eq!(marine.wave_height_m, Some(1.34));
        assert_eq!(marine.wave_direction_deg, Some(245));
        assert_eq!(marine.wave_period_s, Some(6.8));
        assert_eq!(marine.swell_height_m, Some(0.92));
        assert!(parse_marine(&json!({}), "x").is_err());
    }
}
