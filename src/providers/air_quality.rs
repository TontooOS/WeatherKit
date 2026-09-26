use crate::http::get_json;
use crate::types::{AirQuality, AirQualityPoint, PollenLevels, Result, WeatherError};

pub const AIR_API: &str = "https://air-quality-api.open-meteo.com/v1/air-quality";

const FIELDS: &str = "european_aqi,us_aqi,pm10,pm2_5,nitrogen_dioxide,sulphur_dioxide,\
carbon_monoxide,ozone,alder_pollen,birch_pollen,grass_pollen,mugwort_pollen,\
olive_pollen,ragweed_pollen";

/// Fetches air quality from the CAMS Europe domain.
///
/// The CAMS Global model serves as fallback because it covers the whole world
/// but carries fewer pollutants and no pollen outside Europe.
pub fn fetch(lat: f64, lon: f64) -> Result<AirQuality> {
    match fetch_domain(lat, lon, "cams_europe", "CAMS Europe") {
        Ok(quality) => Ok(quality),
        Err(_) => fetch_domain(lat, lon, "cams_global", "CAMS Global"),
    }
}

pub fn fetch_domain(lat: f64, lon: f64, domain: &str, source: &str) -> Result<AirQuality> {
    let url = format!(
        "{}?latitude={:.5}&longitude={:.5}&current={}&domains={}",
        AIR_API, lat, lon, FIELDS, domain
    );
    let json = get_json(&url)?;
    parse(&json, source)
}

/// Builds the hourly AQI forecast URL (keyless, same CAMS domains).
pub fn forecast_url(lat: f64, lon: f64, domain: &str, days: u8) -> String {
    format!(
        "{}?latitude={:.5}&longitude={:.5}\
&hourly=european_aqi,us_aqi,pm2_5,pm10,ozone&forecast_days={}&domains={}",
        AIR_API, lat, lon, days.max(1), domain
    )
}

/// Parses hourly AQI arrays into [`AirQualityPoint`] values.
pub fn parse_forecast(json: &serde_json::Value, limit: usize) -> Result<Vec<AirQualityPoint>> {
    let hourly = json
        .get("hourly")
        .ok_or_else(|| WeatherError::ParseError("air-quality response missing hourly".into()))?;
    let empty = Vec::new();
    let times = hourly
        .get("time")
        .and_then(|v| v.as_array())
        .unwrap_or(&empty);
    let eu = hourly
        .get("european_aqi")
        .and_then(|v| v.as_array())
        .unwrap_or(&empty);
    let us = hourly
        .get("us_aqi")
        .and_then(|v| v.as_array())
        .unwrap_or(&empty);
    let pm25 = hourly
        .get("pm2_5")
        .and_then(|v| v.as_array())
        .unwrap_or(&empty);
    let pm10 = hourly
        .get("pm10")
        .and_then(|v| v.as_array())
        .unwrap_or(&empty);
    let ozone = hourly
        .get("ozone")
        .and_then(|v| v.as_array())
        .unwrap_or(&empty);

    let mut points = Vec::with_capacity(limit.min(times.len()));
    for index in 0..times.len() {
        if points.len() >= limit {
            break;
        }
        points.push(AirQualityPoint {
            time: times[index].as_str().unwrap_or_default().to_string(),
            european_aqi: eu.get(index).and_then(|v| v.as_i64()).map(|v| v as i32),
            us_aqi: us.get(index).and_then(|v| v.as_i64()).map(|v| v as i32),
            pm2_5: pm25.get(index).and_then(|v| v.as_f64()),
            pm10: pm10.get(index).and_then(|v| v.as_f64()),
            ozone: ozone.get(index).and_then(|v| v.as_f64()),
        });
    }

    Ok(points)
}

/// Hourly AQI forecast; CAMS Europe first, CAMS Global as fallback.
pub fn fetch_forecast(lat: f64, lon: f64, hours: usize) -> Result<Vec<AirQualityPoint>> {
    let days = ((hours + 23) / 24).clamp(1, 7) as u8;
    match get_json(&forecast_url(lat, lon, "cams_europe", days))
        .map_err(|e| e.to_string())
        .and_then(|json| parse_forecast(&json, hours).map_err(|e| e.to_string()))
    {
        Ok(points) => Ok(points),
        Err(_) => {
            let json = get_json(&forecast_url(lat, lon, "cams_global", days))?;
            parse_forecast(&json, hours)
        }
    }
}

fn num(value: &serde_json::Value, key: &str) -> Option<f64> {
    value.get(key)?.as_f64()
}

fn int(value: &serde_json::Value, key: &str) -> Option<i32> {
    value.get(key)?.as_i64().map(|v| v as i32)
}

/// Parses an air-quality response.
pub fn parse(json: &serde_json::Value, source: &str) -> Result<AirQuality> {
    let current = json
        .get("current")
        .ok_or_else(|| WeatherError::ParseError("air-quality response missing current".into()))?;

    Ok(AirQuality {
        european_aqi: int(current, "european_aqi"),
        us_aqi: int(current, "us_aqi"),
        pm10: num(current, "pm10"),
        pm2_5: num(current, "pm2_5"),
        ozone: num(current, "ozone"),
        nitrogen_dioxide: num(current, "nitrogen_dioxide"),
        sulphur_dioxide: num(current, "sulphur_dioxide"),
        carbon_monoxide: num(current, "carbon_monoxide"),
        pollen: PollenLevels {
            alder: num(current, "alder_pollen"),
            birch: num(current, "birch_pollen"),
            grass: num(current, "grass_pollen"),
            mugwort: num(current, "mugwort_pollen"),
            olive: num(current, "olive_pollen"),
            ragweed: num(current, "ragweed_pollen"),
        },
        source: source.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn builds_urls() {
        let url = format!(
            "{}?latitude={:.5}&longitude={:.5}&current={}&domains={}",
            AIR_API,
            52.52,
            13.405,
            FIELDS,
            "cams_europe"
        );
        assert!(url.starts_with("https://air-quality-api.open-meteo.com"));
        assert!(url.contains("domains=cams_europe"));
        assert!(url.contains("birch_pollen"));
    }

    #[test]
    fn parses_air_quality() {
        let sample = json!({
            "current": {
                "european_aqi": 32,
                "us_aqi": 41,
                "pm10": 12.4,
                "pm2_5": 7.1,
                "ozone": 68.3,
                "nitrogen_dioxide": 11.9,
                "sulphur_dioxide": 2.0,
                "carbon_monoxide": 120.5,
                "alder_pollen": 0.1,
                "birch_pollen": 14.8,
                "grass_pollen": 3.2,
                "mugwort_pollen": null,
                "olive_pollen": null,
                "ragweed_pollen": 0.4
            }
        });

        let air = parse(&sample, "CAMS Europe").unwrap();
        assert_eq!(air.european_aqi, Some(32));
        assert_eq!(air.us_aqi, Some(41));
        assert_eq!(air.pm2_5, Some(7.1));
        assert_eq!(air.ozone, Some(68.3));
        assert_eq!(air.pollen.birch, Some(14.8));
        assert_eq!(air.pollen.mugwort, None);
        assert_eq!(air.source, "CAMS Europe");
    }

    #[test]
    fn rejects_broken_air_quality() {
        assert!(parse(&json!({}), "x").is_err());
    }

    #[test]
    fn parses_aqi_forecast() {
        let sample = json!({
            "hourly": {
                "time": ["2026-08-24T12:00", "2026-08-24T13:00"],
                "european_aqi": [32, 45],
                "us_aqi": [41, 55],
                "pm2_5": [7.1, 9.4],
                "pm10": [12.4, 15.0],
                "ozone": [68.3, 71.2]
            }
        });

        let points = parse_forecast(&sample, 5).unwrap();
        assert_eq!(points.len(), 2);
        assert_eq!(points[0].european_aqi, Some(32));
        assert_eq!(points[1].us_aqi, Some(55));
        assert_eq!(points[1].pm2_5, Some(9.4));

        assert!(parse_forecast(&json!({}), 2).is_err());
    }
}
