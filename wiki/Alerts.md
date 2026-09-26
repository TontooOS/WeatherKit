# Alerts

Advisory weather alerts with severity levels. Official MET Norway MetAlerts
come first (keyless, Europe coverage); locally synthesized advisories from
current and forecast data act as the global keyless fallback. Both sources
need no API key.

## active_alerts

```rust
pub fn active_alerts(&self) -> Result<Vec<WeatherAlert>>
```

Returns the merged alert list. Official alerts win on duplicate kinds, so a
local synthesis never double-reports what MetAlerts already covers. An empty
list is a valid answer in calm weather or outside MetAlerts coverage.

```rust
pub struct WeatherAlert {
    pub headline: String,
    pub description: String,
    pub severity: AlertSeverity,
    pub kind: String,
    pub source: String,
    pub effective: Option<String>,
    pub expires: Option<String>,
}
```

| Field | Type | Description |
|---|---|---|
| `headline` | `String` | Short title, e.g. `"Storm warning"` |
| `description` | `String` | Details with measured values |
| `severity` | `AlertSeverity` | `Minor`, `Moderate`, `Severe` or `Extreme` |
| `kind` | `String` | `"Storm"`, `"HeavyRain"`, `"Thunderstorm"`, `"Heat"`, `"Frost"`, `"Ultraviolet"`, `"Fog"` |
| `source` | `String` | `"MET Norway MetAlerts"` or `"WeatherKit Synthesis (keyless)"` |
| `effective` / `expires` | `Option<String>` | Validity window (official alerts only) |

## Severity and push notifications

```rust
pub fn should_notify(&self) -> bool
```

Returns `true` for `Moderate` and higher. The system triggers a push
notification exactly when `should_notify` is `true`; `Minor` alerts stay
silent in the alert list.

| Severity | Meaning | Push |
|---|---|---|
| `Minor` | Advisory, no action needed | no |
| `Moderate` | Prepare, check forecast | yes |
| `Severe` | Act, avoid exposure | yes |
| `Extreme` | Danger to life and property | yes |

## Synthesis thresholds

The offline synthesis derives alerts from keyless data only:

| Kind | Moderate | Severe / Extreme |
|---|---|---|
| `Storm` / `Wind` | wind 60 km/h | 75 km/h (`Severe`), 100 km/h (`Extreme`) |
| `HeavyRain` | 7.5 mm/h or 25 mm/day | 15 mm/h or 50 mm/day (`Severe`) |
| `Thunderstorm` | WMO code 95, 96, 99 in hourly/daily | - |
| `Heat` | max 32 C | 38 C (`Severe`) |
| `Frost` | min -5 C | -15 C (`Severe`) |
| `Ultraviolet` | - | UV 9 (`Severe`); UV 6 reports `Minor` |
| `Fog` | visibility 200 m | below 1000 m reports `Minor` |

- Returns `Err(WeatherError::LocationFailed)` when CoreLocation has no fix.
- Official alerts carry `effective` and `expires`; synthesized ones use `None`.
- C FFI: `tontoo_weatherkit_active_alerts` returns the list as a JSON array.

## Async API

```rust
pub async fn active_alerts_async(&self) -> Result<Vec<WeatherAlert>>
```

## Usage / Example

```rust
use weatherkit::WeatherKit;

let kit = WeatherKit::new();
for alert in kit.active_alerts().unwrap() {
    println!("[{}] {} ({})", alert.severity, alert.headline, alert.source);
    if alert.should_notify() {
        println!("push: {}", alert.description);
    }
}
```

## Cross References

- [Forecast.md](Forecast.md) – hourly and daily data feeding the synthesis
- [Current.md](Current.md) – current conditions feeding the synthesis
- [Environment.md](Environment.md) – same fallback model
