# Tontoo WeatherKit

A Framework for Weather Data with 20 Retrieval Functions, Keyless APIs and a
Fallback Source for every Function.

## Made for TontooOS

Explore more at https://github.com/TontooOS/Libs

## Adding to Your Project

Add to your `Cargo.toml`:

```toml
[dependencies]
sdk = { path = "/Library/System/sdk", features = ["WeatherKit"] }
```

## Sources

All sources are keyless: Open-Meteo (forecast, archive, marine, air quality,
geocoding), wttr.in, MET Norway and Nominatim. Every function has one
fallback source; sun and moon run fully offline.

## License

TCL v26.1
