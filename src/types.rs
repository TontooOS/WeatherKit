use crate::lang;
use std::fmt;

#[derive(Debug)]
pub enum WeatherError {
    NotAvailable,
    PermissionDenied,
    Timeout,
    NetworkError(String),
    ParseError(String),
    ProviderFailed(String),
    LocationFailed(String),
}

impl fmt::Display for WeatherError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WeatherError::NotAvailable => write!(f, "{}", lang::t("not_available")),
            WeatherError::PermissionDenied => write!(f, "{}", lang::t("permission_denied")),
            WeatherError::Timeout => write!(f, "{}", lang::t("timeout")),
            WeatherError::NetworkError(e) => write!(f, "{}", lang::t_fmt("network_error", e)),
            WeatherError::ParseError(e) => write!(f, "{}", lang::t_fmt("parse_error", e)),
            WeatherError::ProviderFailed(p) => write!(f, "{}", lang::t_fmt("provider_failed", p)),
            WeatherError::LocationFailed(l) => write!(f, "{}", lang::t_fmt("location_failed", l)),
        }
    }
}

impl std::error::Error for WeatherError {}

pub type Result<T> = std::result::Result<T, WeatherError>;

#[derive(Debug, Clone)]
pub struct CurrentWeather {
    pub temperature_c: f64,
    pub feels_like_c: f64,
    pub humidity_pct: i32,
    pub pressure_hpa: f64,
    pub wind_kmh: f64,
    pub wind_direction_deg: i32,
    pub wind_gusts_kmh: Option<f64>,
    pub precipitation_mm: f64,
    pub cloud_cover_pct: Option<i32>,
    pub visibility_m: Option<f64>,
    pub uv_index: Option<f64>,
    pub is_day: bool,
    pub weather_code: Option<i32>,
    pub condition: String,
    pub source: String,
}

impl Default for CurrentWeather {
    fn default() -> Self {
        Self {
            temperature_c: 0.0,
            feels_like_c: 0.0,
            humidity_pct: 0,
            pressure_hpa: 1013.25,
            wind_kmh: 0.0,
            wind_direction_deg: 0,
            wind_gusts_kmh: None,
            precipitation_mm: 0.0,
            cloud_cover_pct: None,
            visibility_m: None,
            uv_index: None,
            is_day: true,
            weather_code: None,
            condition: "Unknown".to_string(),
            source: String::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct HourPoint {
    pub time: String,
    pub temperature_c: f64,
    pub precipitation_mm: f64,
    pub precip_probability_pct: Option<i32>,
    pub wind_kmh: f64,
    pub weather_code: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct ForecastDay {
    pub date: String,
    pub temp_max_c: f64,
    pub temp_min_c: f64,
    pub precipitation_mm: Option<f64>,
    pub precip_probability_pct: Option<i32>,
    pub wind_max_kmh: f64,
    pub weather_code: Option<i32>,
    pub sunrise_utc: Option<String>,
    pub sunset_utc: Option<String>,
}

#[derive(Debug, Clone)]
pub struct HistoricalDay {
    pub date: String,
    pub temp_max_c: Option<f64>,
    pub temp_min_c: Option<f64>,
    pub precipitation_mm: Option<f64>,
}

#[derive(Debug, Clone, Default)]
pub struct PollenLevels {
    pub alder: Option<f64>,
    pub birch: Option<f64>,
    pub grass: Option<f64>,
    pub mugwort: Option<f64>,
    pub olive: Option<f64>,
    pub ragweed: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct AirQuality {
    pub european_aqi: Option<i32>,
    pub us_aqi: Option<i32>,
    pub pm10: Option<f64>,
    pub pm2_5: Option<f64>,
    pub ozone: Option<f64>,
    pub nitrogen_dioxide: Option<f64>,
    pub sulphur_dioxide: Option<f64>,
    pub carbon_monoxide: Option<f64>,
    pub pollen: PollenLevels,
    pub source: String,
}

#[derive(Debug, Clone)]
pub struct MarineConditions {
    pub wave_height_m: Option<f64>,
    pub wave_direction_deg: Option<i32>,
    pub wave_period_s: Option<f64>,
    pub swell_height_m: Option<f64>,
    pub source: String,
}

#[derive(Debug, Clone)]
pub struct SunTimes {
    pub sunrise: (u32, u32),
    pub sunset: (u32, u32),
    pub day_length_secs: u64,
    pub polar_day: bool,
    pub polar_night: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MoonPhase {
    pub age_days: f64,
    pub phase_fraction: f64,
    pub illumination_pct: f64,
    pub name: &'static str,
}

#[derive(Debug, Clone)]
pub struct Place {
    pub name: String,
    pub country: Option<String>,
    pub region: Option<String>,
    pub latitude: f64,
    pub longitude: f64,
}

/// WMO weather interpretation codes (WW codes used by Open-Meteo).
pub fn weather_code_description(code: i32) -> &'static str {
    match code {
        0 => "Clear sky",
        1 => "Mainly clear",
        2 => "Partly cloudy",
        3 => "Overcast",
        45 | 48 => "Fog",
        51 | 53 | 55 => "Drizzle",
        56 | 57 => "Freezing drizzle",
        61 | 63 | 65 => "Rain",
        66 | 67 => "Freezing rain",
        71 | 73 | 75 => "Snowfall",
        77 => "Snow grains",
        80 | 81 | 82 => "Rain showers",
        85 | 86 => "Snow showers",
        95 => "Thunderstorm",
        96 | 99 => "Thunderstorm with hail",
        _ => "Unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_messages_localized() {
        assert_eq!(
            WeatherError::NotAvailable.to_string(),
            lang::t("not_available")
        );
        assert_eq!(
            WeatherError::ProviderFailed("x".into()).to_string(),
            lang::t_fmt("provider_failed", "x")
        );
    }

    #[test]
    fn wmo_codes_covered() {
        assert_eq!(weather_code_description(0), "Clear sky");
        assert_eq!(weather_code_description(3), "Overcast");
        assert_eq!(weather_code_description(95), "Thunderstorm");
        assert_eq!(weather_code_description(42), "Unknown");
        assert_eq!(weather_code_description(-1), "Unknown");
    }
}
