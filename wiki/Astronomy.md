# Astronomy

Sun and moon calculations run completely locally from the CoreLocation
coordinates. They need no network, no API key and no fallback source, which
makes them the only weather functions that always work offline.

## sun_times

```rust
pub fn sun_times(&self, year: i32, month: u32, day: u32) -> Result<SunTimes>
```

Implements the standard sunrise equation (mean anomaly, equation of center,
ecliptic longitude, declination, hour angle at -0.83 degrees).

```rust
pub struct SunTimes {
    pub sunrise: (u32, u32),
    pub sunset: (u32, u32),
    pub day_length_secs: u64,
    pub polar_day: bool,
    pub polar_night: bool,
}
```

- Times are UTC hours/minutes; no timezone database is involved.
- Polar regions are explicit instead of undefined:
  - `polar_day = true` reports `00:00` to `23:59` with a full day length.
  - `polar_night = true` reports `12:00` twice with a day length of zero.

## is_daytime

```rust
pub fn is_daytime(lat: f64, lon: f64) -> bool
```

Standalone function used by [`is_day`](Current.md#is_day) as network-free
fallback. It computes today's sun times for the coordinates and compares the
current UTC time against them; polar cases are honored.

## moon_phase

```rust
pub fn moon_phase(&self) -> MoonPhase
```

Computes the moon age from the synodic month (29.530588853 days) since the
new moon of January 2000.

```rust
pub struct MoonPhase {
    pub age_days: f64,
    pub phase_fraction: f64,
    pub illumination_pct: f64,
    pub name: &'static str,
}
```

| `phase_fraction` | Name |
|---|---|
| `< 0.02` or `>= 0.98` | New Moon |
| `< 0.22` | Waxing Crescent |
| `< 0.28` | First Quarter |
| < 0.47 | Waxing Gibbous |
| < 0.53 | Full Moon |
| < 0.72 | Waning Gibbous |
| < 0.78 | Last Quarter |
| otherwise | Waning Crescent |

Illumination follows `(1 - cos(2π * fraction)) / 2`.

## Pure functions

The astronomy module exposes its math for reuse:

```rust
pub fn julian_day(year: i32, month: u32, day: u32) -> f64
pub fn civil_from_days(days_since_epoch: i64) -> (i32, u32, u32)
pub fn now_julian_day() -> f64
```

- `julian_day` returns the JD at 00:00 UT of the date.
- `civil_from_days` converts Unix days into a civil date (Hinnant's
  algorithm) without any date library.

## Usage / Example

```rust
use weatherkit::WeatherKit;

let kit = WeatherKit::new();
let times = kit.sun_times(2026, 8, 24).unwrap();
println!("{:02}:{:02} - {:02}:{:02} UTC", times.sunrise.0,
    times.sunrise.1, times.sunset.0, times.sunset.1);

let moon = kit.moon_phase();
println!("{} ({:.0}% illuminated)", moon.name, moon.illumination_pct);
```

## Cross References

- [Current.md](Current.md) – `is_day` fallback chain
