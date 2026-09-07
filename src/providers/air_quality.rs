use crate::http::get_json;
use crate::types::{AirQuality, PollenLevels, Result, WeatherError};

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
}
