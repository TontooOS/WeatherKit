# Environment

Environmental data beyond plain weather: the ERA5 based air quality models of
CAMS and ocean wave conditions. Both groups use keyless endpoints with a
domain or provider fallback.

## air_quality

```rust
pub fn air_quality(&self) -> Result<AirQuality>
```

Queries `air-quality-api.open-meteo.com` for the current hour.

| Field | Type | Description |
|---|---|---|
| `european_aqi` | `Option<i32>` | European AQI (0-100+) |
| `us_aqi` | `Option<i32>` | US EPA AQI |
| `pm10` / `pm2_5` | `Option<f64>` | Particulate matter in µg/m³ |
| `ozone` | `Option<f64>` | O₃ in µg/m³ |
| `nitrogen_dioxide` | `Option<f64>` | NO₂ in µg/m³ |
| `sulphur_dioxide` | `Option<f64>` | SO₂ in µg/m³ |
| `carbon_monoxide` | `Option<f64>` | CO in µg/m³ |
| `pollen` | `PollenLevels` | See below |
| `source` | `String` | `"CAMS Europe"` or `"CAMS Global"` |

Fallback behavior: the request runs against the CAMS Europe domain first.
When it fails, the same query repeats with `domains=cams_global`. The global
model covers every location worldwide but carries fewer pollutants and no
pollen.

## pollen

```rust
pub fn pollen(&self) -> Result<PollenLevels>
```

Extracts the pollen block of [`air_quality`](#airquality). Concentrations are
grains per cubic meter:

```rust
pub struct PollenLevels {
    pub alder: Option<f64>,
    pub birch: Option<f64>,
    pub grass: Option<f64>,
    pub mugwort: Option<f64>,
    pub olive: Option<f64>,
    pub ragweed: Option<f64>,
}
```

Outside Europe every field is typically `None`; that is a data limitation of
CAMS, not an error.

## marine_conditions

```rust
pub fn marine_conditions(&self) -> Result<MarineConditions>
```

Wave conditions near coastal locations.

```rust
pub struct MarineConditions {
    pub wave_height_m: Option<f64>,
    pub wave_direction_deg: Option<i32>,
    pub wave_period_s: Option<f64>,
    pub swell_height_m: Option<f64>,
    pub source: String,
}
```

- Primary source: Open-Meteo Marine (`marine-api.open-meteo.com`, ECMWF WAM).
  Inland locations answer successfully but with `None` fields because no grid
  cell exists.
- Fallback source: MET Norway oceanforecast; it fills its own detail keys and
  reports `source = "MET Norway Ocean"`.
- Returns `Err(WeatherError::ProviderFailed)` only when both providers fail.

## Async API

```rust
pub async fn air_quality_async(&self) -> Result<AirQuality>
```

## Usage / Example

```rust
use weatherkit::WeatherKit;

let kit = WeatherKit::new();
let air = kit.air_quality().unwrap();
println!("EU AQI {:?} ({})", air.european_aqi, air.source);

let pollen = kit.pollen().unwrap();
println!("birch {:?}", pollen.birch);
```

## Cross References

- [Forecast.md](Forecast.md) – same cache model
- [Current.md](Current.md) – location resolution via CoreLocation
