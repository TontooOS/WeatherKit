# Forecast

Forecast functions come in two flavors: hourly points for the next hours and
aggregated calendar days. Open-Meteo is the primary source, the MET Norway
compact locationforecast is the fallback; both are keyless.

## hourly_forecast

```rust
pub fn hourly_forecast(&self, hours: usize) -> Result<Vec<HourPoint>>
```

Returns the next `hours` entries (at least one). Cached per coordinate and
hour count for thirty minutes.

```rust
pub struct HourPoint {
    pub time: String,
    pub temperature_c: f64,
    pub precipitation_mm: f64,
    pub precip_probability_pct: Option<i32>,
    pub wind_kmh: f64,
    pub weather_code: Option<i32>,
}
```

- Times are ISO stamps with timezone offset (`2026-08-25T14:00`).
- `weather_code` and precipitation probability are only filled by Open-Meteo;
  MET Norway answers keep them `None`.

## daily_forecast

```rust
pub fn daily_forecast(&self, days: usize) -> Result<Vec<ForecastDay>>
```

Returns up to sixteen days (Open-Meteo limit); MET Norway aggregates at most
nine days from its timeseries.

| Field | Type | Description |
|---|---|---|
| `date` | `String` | Calendar day, `YYYY-MM-DD` |
| `temp_max_c` / `temp_min_c` | `f64` | Daily extremes |
| `precipitation_mm` | `Option<f64>` | Daily sum |
| `precip_probability_pct` | `Option<i32>` | Peak probability (Open-Meteo) |
| `wind_max_kmh` | `f64` | Maximum wind speed |
| `weather_code` | `Option<i32>` | WMO code (Open-Meteo) |
| `sunrise_utc` / `sunset_utc` | `Option<String>` | Local sunrise/sunset stamps |

The MET Norway fallback buckets every timeseries entry by date and computes
min/max temperature, total precipitation and peak wind itself.

## historical_weather

```rust
pub fn historical_weather(
    &self,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<HistoricalDay>>
```

Observed daily values between two ISO dates.

- Primary source is the ERA5 archive endpoint.
- Fallback uses the forecast API with `past_days` for recent ranges; it
  covers at most 92 days back and silently filters to the requested window.
- Returns `Err(WeatherError::ProviderFailed)` when a range is out of reach
  for both sources.

```rust
pub struct HistoricalDay {
    pub date: String,
    pub temp_max_c: Option<f64>,
    pub temp_min_c: Option<f64>,
    pub precipitation_mm: Option<f64>,
}
```

`None` fields mean the source had no value for that day.

## Async API

```rust
pub async fn daily_forecast_async(&self, days: usize) -> Result<Vec<ForecastDay>>
```

## Usage / Example

```rust
use weatherkit::WeatherKit;

let kit = WeatherKit::new();
for hour in kit.hourly_forecast(6).unwrap() {
    println!("{} {:.1}°C {:.1}mm", hour.time, hour.temperature_c,
        hour.precipitation_mm);
}

for day in kit.daily_forecast(7).unwrap() {
    println!("{} {:.0}/{:.0}°C", day.date, day.temp_min_c,
        day.temp_max_c);
}
```

## Cross References

- [Current.md](Current.md) – shared cache and location resolution
- [Astronomy.md](Astronomy.md) – offline alternative for sun times
