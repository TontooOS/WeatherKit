# WeatherKit – Wiki

WeatherKit is the weather framework for TontooOS. It provides more than twenty
weather retrieval functions built exclusively on keyless APIs, every function
backed by a fallback source, with location coming only from CoreLocation.

- Repository: https://github.com/TontooOS/Libs
- License: TCL
- Version: 26.1.0

## Feature Index

| Feature | File | Description |
|---|---|---|
| Main index | [MAIN.md](MAIN.md) | This page |
| Rules | [RULE.md](RULE.md) | Development and usage rules |
| Current | [Current.md](Current.md) | `WeatherKit` current conditions and the twelve single value getters |
| Forecast | [Forecast.md](Forecast.md) | Hourly and daily forecasts, historical archive |
| Environment | [Environment.md](Environment.md) | Air quality, pollen, marine conditions |
| Astronomy | [Astronomy.md](Astronomy.md) | Local sunrise/sunset, moon phase, daylight check |
| Places | [Places.md](Places.md) | Place search and weather by place name |
| Localization | [Localization.md](Localization.md) | Error message localization via `lang/en_us.json` and `lang/de_de.json` |

## Quick Start

```rust
use weatherkit::{current_weather, weekly_forecast};

let current = current_weather().unwrap();
println!("{}: {:.1}°C ({})", current.condition,
    current.temperature_c, current.source);

for day in weekly_forecast().unwrap() {
    println!("{} {:.0}/{:.0}°C", day.date, day.temp_min_c, day.temp_max_c);
}
```

See [Current.md](Current.md) and [Forecast.md](Forecast.md) for details.

## Sources and Fallbacks

Every network function has exactly one fallback chain; all sources are
keyless:

| Function group | Primary | Fallback |
|---|---|---|
| Current conditions | Open-Meteo forecast | wttr.in (`format=j1`) |
| Current (last resort) | - | MET Norway locationforecast (degraded) |
| Hourly/daily forecasts | Open-Meteo forecast | MET Norway locationforecast compact |
| Historical | Open-Meteo ERA5 archive | Open-Meteo forecast with `past_days` |
| Air quality / pollen | CAMS Europe domain | CAMS Global domain |
| Marine | Open-Meteo Marine (ECMWF WAM) | MET Norway oceanforecast |
| Places | Open-Meteo geocoding | Nominatim (OpenStreetMap) |
| Astronomy | Local calculation | none needed (offline) |

Location is resolved exclusively through CoreLocation (GPS, WiFi, IP,
timezone); [`WeatherKit::at`](Current.md#weatherkatlat-lon) pins fixed
coordinates instead. Results are cached per rounded coordinate for ten minutes
(current) or thirty minutes (forecasts).

## Changelog

- 2026-08-25: Initial wiki with Current, Forecast, Environment, Astronomy,
  Places and Localization pages.
