pub mod astronomy;
pub mod http;
pub mod lang;
pub mod location;
pub mod providers;
pub mod types;

pub use location::LocationResolver;
pub use types::{
    weather_code_description, AirQuality, CurrentWeather, ForecastDay, HistoricalDay, HourPoint,
    MarineConditions, MoonPhase, Place, PollenLevels, Result, SunTimes, WeatherError,
};

use location::LocationResolver as Resolver;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const CURRENT_TTL: Duration = Duration::from_secs(600);
const FORECAST_TTL: Duration = Duration::from_secs(1800);

#[derive(Clone)]
struct CacheEntry<T> {
    key: String,
    at: Instant,
    value: T,
}

impl<T: Clone> CacheEntry<T> {
    fn get(&self, key: &str, ttl: Duration) -> Option<T> {
        if self.key == key && self.at.elapsed() < ttl {
            Some(self.value.clone())
        } else {
            None
        }
    }
}

fn cache_key(lat: f64, lon: f64) -> String {
    format!("{:.2},{:.2}", lat, lon)
}

/// The WeatherKit client.
///
/// Location comes exclusively from CoreLocation unless the kit is pinned to
/// coordinates via [`WeatherKit::at`]. Every network function has a fallback
/// source; all sources are keyless.
#[derive(Clone)]
pub struct WeatherKit {
    resolver: Resolver,
    current_cache: Arc<Mutex<Option<CacheEntry<CurrentWeather>>>>,
    hourly_cache: Arc<Mutex<Option<CacheEntry<Vec<HourPoint>>>>>,
    daily_cache: Arc<Mutex<Option<CacheEntry<Vec<ForecastDay>>>>>,
}

impl WeatherKit {
    /// Uses CoreLocation (GPS, WiFi, IP, timezone) for every request.
    pub fn new() -> Self {
        Self {
            resolver: Resolver::auto(),
            current_cache: Arc::new(Mutex::new(None)),
            hourly_cache: Arc::new(Mutex::new(None)),
            daily_cache: Arc::new(Mutex::new(None)),
        }
    }

    /// Pins all requests to fixed coordinates without touching CoreLocation.
    pub fn at(lat: f64, lon: f64) -> Self {
        Self {
            resolver: Resolver::pinned(lat, lon),
            current_cache: Arc::new(Mutex::new(None)),
            hourly_cache: Arc::new(Mutex::new(None)),
            daily_cache: Arc::new(Mutex::new(None)),
        }
    }

    fn coords(&self) -> Result<(f64, f64)> {
        let coords = self.resolver.resolve()?;
        Ok((coords.latitude, coords.longitude))
    }

    // ---- current conditions ----

    fn fetch_current_uncached(coords: (f64, f64)) -> Result<CurrentWeather> {
        providers::open_meteo::fetch_current(coords.0, coords.1)
            .or_else(|_| providers::wttr::fetch_current(coords.0, coords.1))
            .or_else(|_| providers::met_no::fetch_current_degraded(coords.0, coords.1))
    }

    fn cached_current(&self) -> Result<CurrentWeather> {
        let coords = self.coords()?;
        let key = cache_key(coords.0, coords.1);

        if let Ok(cache) = self.current_cache.lock() {
            if let Some(entry) = cache.as_ref() {
                if let Some(value) = entry.get(&key, CURRENT_TTL) {
                    return Ok(value);
                }
            }
        }

        let value = Self::fetch_current_uncached(coords)?;
        if let Ok(mut cache) = self.current_cache.lock() {
            *cache = Some(CacheEntry {
                key,
                at: Instant::now(),
                value: value.clone(),
            });
        }
        Ok(value)
    }

    /// Full current conditions with primary and fallback sources merged in.
    pub fn current_weather(&self) -> Result<CurrentWeather> {
        self.cached_current()
    }

    /// Temperature right now in degrees Celsius.
    pub fn temperature(&self) -> Result<f64> {
        Ok(self.cached_current()?.temperature_c)
    }

    /// Apparent temperature ("feels like") in degrees Celsius.
    pub fn feels_like(&self) -> Result<f64> {
        Ok(self.cached_current()?.feels_like_c)
    }

    /// Relative humidity in percent.
    pub fn humidity(&self) -> Result<i32> {
        Ok(self.cached_current()?.humidity_pct)
    }

    /// Sea level pressure in hPa.
    pub fn pressure(&self) -> Result<f64> {
        Ok(self.cached_current()?.pressure_hpa)
    }

    /// Wind speed in km/h.
    pub fn wind_speed(&self) -> Result<f64> {
        Ok(self.cached_current()?.wind_kmh)
    }

    /// Wind direction in degrees (meteorological, 0 = north).
    pub fn wind_direction(&self) -> Result<i32> {
        Ok(self.cached_current()?.wind_direction_deg)
    }

    /// Wind gusts in km/h when the active source reports them.
    pub fn wind_gusts(&self) -> Result<Option<f64>> {
        Ok(self.cached_current()?.wind_gusts_kmh)
    }

    /// UV index when available; falls back to a conservative clear-sky
    /// estimate from cloud cover and daytime.
    pub fn uv_index(&self) -> Result<f64> {
        match self.cached_current()? {
            weather if weather.uv_index.is_some() => Ok(weather.uv_index.unwrap()),
            weather => Ok(estimate_uv(weather.cloud_cover_pct, weather.is_day)),
        }
    }

    /// Visibility in meters; `None` when the active source has none.
    pub fn visibility(&self) -> Result<Option<f64>> {
        Ok(self.cached_current()?.visibility_m)
    }

    /// Precipitation of the last hour in millimeters.
    pub fn precipitation_now(&self) -> Result<f64> {
        Ok(self.cached_current()?.precipitation_mm)
    }

    /// Whether it is daylight right now.
    ///
    /// Open-Meteo reports daytime directly; every other source falls back to
    /// the local solar calculation which needs no network at all.
    pub fn is_day(&self) -> Result<bool> {
        match self.cached_current() {
            Ok(weather) if weather.source == "Open-Meteo" => Ok(weather.is_day),
            _ => {
                let (lat, lon) = self.coords()?;
                Ok(astronomy::is_daytime(lat, lon))
            }
        }
    }

    // ---- forecasts ----

    fn cached_hourly(&self, hours: usize) -> Result<Vec<HourPoint>> {
        let coords = self.coords()?;
        let key = format!("{}|{}", cache_key(coords.0, coords.1), hours);

        if let Ok(cache) = self.hourly_cache.lock() {
            if let Some(entry) = cache.as_ref() {
                if let Some(value) = entry.get(&key, FORECAST_TTL) {
                    return Ok(value);
                }
            }
        }

        let mut value =
            providers::open_meteo::fetch_hourly(coords.0, coords.1, hours).or_else(|_| {
                providers::met_no::fetch_hourly(coords.0, coords.1, hours)
            })?;
        value.truncate(hours);

        if let Ok(mut cache) = self.hourly_cache.lock() {
            *cache = Some(CacheEntry {
                key,
                at: Instant::now(),
                value: value.clone(),
            });
        }
        Ok(value)
    }

    /// Hourly forecast points for the next `hours` hours.
    pub fn hourly_forecast(&self, hours: usize) -> Result<Vec<HourPoint>> {
        self.cached_hourly(hours.max(1))
    }

    fn cached_daily(&self, days: usize) -> Result<Vec<ForecastDay>> {
        let coords = self.coords()?;
        let key = format!("{}|{}", cache_key(coords.0, coords.1), days);

        if let Ok(cache) = self.daily_cache.lock() {
            if let Some(entry) = cache.as_ref() {
                if let Some(value) = entry.get(&key, FORECAST_TTL) {
                    return Ok(value);
                }
            }
        }

        let mut value = providers::open_meteo::fetch_daily(
            coords.0,
            coords.1,
            days.min(16) as u8,
        )
        .or_else(|_| {
            providers::met_no::fetch_daily(coords.0, coords.1, days.min(9))
        })?;
        value.truncate(days);

        if let Ok(mut cache) = self.daily_cache.lock() {
            *cache = Some(CacheEntry {
                key,
                at: Instant::now(),
                value: value.clone(),
            });
        }
        Ok(value)
    }

    /// Daily forecast for the next `days` days (max 16).
    pub fn daily_forecast(&self, days: usize) -> Result<Vec<ForecastDay>> {
        self.cached_daily(days.clamp(1, 16))
    }

    // ---- history and environment ----

    /// Observed daily values between two ISO dates (`YYYY-MM-DD`).
    ///
    /// Primary source is the ERA5 archive; recent ranges fall back to the
    /// forecast API with past days included.
    pub fn historical_weather(
        &self,
        start_date: &str,
        end_date: &str,
    ) -> Result<Vec<HistoricalDay>> {
        let (lat, lon) = self.coords()?;
        providers::open_meteo::fetch_historical(lat, lon, start_date, end_date).or_else(|_| {
            let days_back = days_until_today(end_date).unwrap_or(7).clamp(1, 92);
            let json_url = format!(
                "{}?latitude={:.5}&longitude={:.5}\
&hourly=temperature_2m&past_days={}\
&daily=temperature_2m_max,temperature_2m_min,precipitation_sum&timezone=auto",
                providers::open_meteo::FORECAST_API,
                lat,
                lon,
                days_back
            );
            let json = crate::http::get_json(&json_url)?;
            providers::open_meteo::parse_daily(&json, usize::MAX)
                .map(|days| {
                    days.into_iter()
                        .filter(|day| day.date.as_str() >= start_date)
                        .map(|day| HistoricalDay {
                            date: day.date,
                            temp_max_c: Some(day.temp_max_c),
                            temp_min_c: Some(day.temp_min_c),
                            precipitation_mm: day.precipitation_mm,
                        })
                        .collect()
                })
        })
    }

    /// Air quality indices and pollutant concentrations.
    pub fn air_quality(&self) -> Result<AirQuality> {
        let (lat, lon) = self.coords()?;
        providers::air_quality(lat, lon)
    }

    /// Pollen concentrations in grains per cubic meter.
    ///
    /// Only meaningful in Europe where CAMS Europe provides pollen data; other
    /// regions return an empty [`PollenLevels`].
    pub fn pollen(&self) -> Result<PollenLevels> {
        Ok(self.air_quality()?.pollen)
    }

    /// Wave conditions near coastal locations.
    pub fn marine_conditions(&self) -> Result<MarineConditions> {
        let (lat, lon) = self.coords()?;
        providers::marine(lat, lon)
    }

    // ---- astronomy (fully local) ----

    /// Sunrise/sunset in UTC computed locally from CoreLocation coordinates.
    pub fn sun_times(&self, year: i32, month: u32, day: u32) -> Result<SunTimes> {
        let (lat, lon) = self.coords()?;
        Ok(astronomy::sun_times(lat, lon, year, month, day))
    }

    /// Current moon phase computed locally, no network involved.
    pub fn moon_phase(&self) -> MoonPhase {
        astronomy::moon_phase()
    }

    // ---- places ----

    /// Searches places by name; Open-Meteo geocoding first, Nominatim second.
    pub fn search_places(&self, query: &str) -> Result<Vec<Place>> {
        providers::places(query, 8)
    }

    /// Current weather for a place name instead of CoreLocation coordinates.
    pub fn weather_for_place(&self, query: &str) -> Result<CurrentWeather> {
        let place = self
            .search_places(query)?
            .into_iter()
            .next()
            .ok_or_else(|| WeatherError::ProviderFailed(format!("no results for {}", query)))?;

        Self::fetch_current_uncached((place.latitude, place.longitude))
    }

    // ---- async wrappers ----

    pub async fn current_weather_async(&self) -> Result<CurrentWeather> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || this.current_weather())
            .await
            .map_err(join_error)?
    }

    pub async fn daily_forecast_async(&self, days: usize) -> Result<Vec<ForecastDay>> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || this.daily_forecast(days))
            .await
            .map_err(join_error)?
    }

    pub async fn air_quality_async(&self) -> Result<AirQuality> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || this.air_quality())
            .await
            .map_err(join_error)?
    }

    pub async fn weather_for_place_async(&self, query: &str) -> Result<CurrentWeather> {
        let this = self.clone();
        let query = query.to_string();
        tokio::task::spawn_blocking(move || this.weather_for_place(&query))
            .await
            .map_err(join_error)?
    }
}

fn join_error(err: tokio::task::JoinError) -> WeatherError {
    WeatherError::NetworkError(err.to_string())
}

fn estimate_uv(cloud_cover_pct: Option<i32>, is_day: bool) -> f64 {
    if !is_day {
        return 0.0;
    }
    let base = 6.0;
    match cloud_cover_pct {
        None => base / 2.0,
        Some(cloud) => base * (1.0 - cloud as f64 / 100.0).max(0.15),
    }
}

fn days_until_today(date: &str) -> Option<i64> {
    let parts: Vec<&str> = date.split('-').collect();
    if parts.len() != 3 {
        return None;
    }
    let year: i64 = parts[0].parse().ok()?;
    let month: i64 = parts[1].parse().ok()?;
    let day: i64 = parts[2].parse().ok()?;

    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs() as i64;
    let today_days = secs / 86400;

    let target_days = days_from_civil(year, month, day);
    Some(today_days - target_days)
}

/// Howard Hinnant's days_from_civil algorithm.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

impl Default for WeatherKit {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience wrapper matching the free-function style of CoreLocation.
pub fn current_weather() -> Result<CurrentWeather> {
    WeatherKit::new().current_weather()
}

/// Convenience wrapper returning only the current temperature.
pub fn temperature() -> Result<f64> {
    WeatherKit::new().temperature()
}

/// Convenience wrapper for the seven day forecast.
pub fn weekly_forecast() -> Result<Vec<ForecastDay>> {
    WeatherKit::new().daily_forecast(7)
}

mod ffi;
