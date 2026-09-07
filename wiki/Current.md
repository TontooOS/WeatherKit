# Current

The `WeatherKit` client resolves a fix through CoreLocation once, then answers
current condition queries from one shared fetch with a ten minute cache.
Open-Meteo serves as primary source, wttr.in as fallback and MET Norway as
last resort.

## Constructors

```rust
pub fn new() -> Self
pub fn at(lat: f64, lon: f64) -> Self
```

- `new` uses CoreLocation exclusively for location (GPS, WiFi, IP, timezone).
- `at` pins fixed coordinates and never touches CoreLocation; useful for
  tests and widgets.

## current_weather

```rust
pub fn current_weather(&self) -> Result<CurrentWeather>
```

Fetches everything in one request and caches it per rounded coordinate
(`0.01°`, roughly one kilometer).

| Field | Type | Description |
|---|---|---|
| `temperature_c` | `f64` | Air temperature at two meters |
| `feels_like_c` | `f64` | Apparent temperature |
| `humidity_pct` | `i32` | Relative humidity |
| `pressure_hpa` | `f64` | Sea level pressure, surface pressure fallback |
| `wind_kmh` | `f64` | Wind speed at ten meters |
| `wind_direction_deg` | `i32` | Meteorological direction, 0 = north |
| `wind_gusts_kmh` | `Option<f64>` | Gust speed when the source reports it |
| `precipitation_mm` | `f64` | Precipitation of the last hour |
| `cloud_cover_pct` | `Option<i32>` | Total cloud cover |
| `visibility_m` | `Option<f64>` | Visibility in meters (wttr.in only) |
| `uv_index` | `Option<f64>` | UV index when the source reports it |
| `is_day` | `bool` | Daylight flag from the source |
| `weather_code` | `Option<i32>` | WMO code (Open-Meteo only) |
| `condition` | `String` | Human readable description |
| `source` | `String` | Which provider answered |

Returns `Err(WeatherError::ProviderFailed)` only when all three sources fail;
the error text names both failures of the chain.

## Single value getters

Each getter reads from the same cached [`current_weather`](#currentweather):

```rust
pub fn temperature(&self) -> Result<f64>
pub fn feels_like(&self) -> Result<f64>
pub fn humidity(&self) -> Result<i32>
pub fn pressure(&self) -> Result<f64>
pub fn wind_speed(&self) -> Result<f64>
pub fn wind_direction(&self) -> Result<i32>
pub fn wind_gusts(&self) -> Result<Option<f64>>
pub fn uv_index(&self) -> Result<f64>
pub fn visibility(&self) -> Result<Option<f64>>
pub fn precipitation_now(&self) -> Result<f64>
```

Special behavior:

- `uv_index` never fails on missing data: without a reported index it derives
  an estimate from cloud cover and daytime (`0.0` at night).
- `visibility` returns `Ok(None)` when the active source has no visibility
  data (only wttr.in provides it).
- `wind_gusts` returns `Ok(None)` when the source reports no gusts.

## is_day

```rust
pub fn is_day(&self) -> Result<bool>
```

Trusts the daylight flag only from Open-Meteo. Every other source falls back
to the pure local solar calculation from [Astronomy.md](Astronomy.md), so the
answer stays correct even when wttr.in (which has no night flag) answered.

## Async API

```rust
pub async fn current_weather_async(&self) -> Result<CurrentWeather>
```

Runs the blocking chain inside `tokio::task::spawn_blocking`.

## Usage / Example

```rust
use weatherkit::WeatherKit;

let kit = WeatherKit::new();
let weather = kit.current_weather().unwrap();
println!("{} ({})", weather.condition, weather.source);
println!("{:.1}°C, feels {:.1}°C", weather.temperature_c,
    weather.feels_like_c);
println!("humidity {}%, wind {:.1} km/h", weather.humidity_pct,
    weather.wind_kmh);
```

## Cross References

- [Forecast.md](Forecast.md) – hourly and daily outlooks
- [Astronomy.md](Astronomy.md) – local daylight calculation behind `is_day`
