# Places

Place search decouples weather requests from CoreLocation coordinates. Two
keyless geocoders are chained: Open-Meteo geocoding first, the OpenStreetMap
Nominatim second.

## search_places

```rust
pub fn search_places(&self, query: &str) -> Result<Vec<Place>>
```

Searches by name and returns up to eight results.

```rust
pub struct Place {
    pub name: String,
    pub country: Option<String>,
    pub region: Option<String>,
    pub latitude: f64,
    pub longitude: f64,
}
```

- Open-Meteo fills `country` and `region` (`admin1`) directly.
- Nominatim splits `display_name`; `region` becomes the second to last
  segment, `country` stays `None`.
- Queries are percent-encoded, so names like `New York` or `München` work.
- Returns `Err(WeatherError::ProviderFailed)` only when both geocoders fail;
  an empty result set from Open-Meteo automatically triggers Nominatim.

## weather_for_place

```rust
pub fn weather_for_place(&self, query: &str) -> Result<CurrentWeather>
```

Resolves the first search result and fetches its current conditions with the
full fallback chain from [Current.md](Current.md). The returned `source`
field still names the actual weather provider; CoreLocation is never used on
this path.

Returns `Err(WeatherError::ProviderFailed)` when no place matched.

## Async API

```rust
pub async fn weather_for_place_async(&self, query: &str) -> Result<CurrentWeather>
```

## Usage / Example

```rust
use weatherkit::WeatherKit;

let kit = WeatherKit::new();

for place in kit.search_places("Hamburg").unwrap() {
    println!("{} ({:.4}, {:.4})", place.name,
        place.latitude, place.longitude);
}

let weather = kit.weather_for_place("Tokyo").unwrap();
println!("Tokyo: {:.1}°C", weather.temperature_c);
```

## Cross References

- [Current.md](Current.md) – fetch chain behind `weather_for_place`
