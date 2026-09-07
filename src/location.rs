use crate::types::{Result, WeatherError};
use corelocation::Coordinates;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const CACHE_TTL: Duration = Duration::from_secs(300);

/// Resolves coordinates exclusively through CoreLocation.
///
/// Fixes are cached for five minutes so bursty calls do not re-run the
/// location pipeline on every weather request.
#[derive(Clone)]
pub struct LocationResolver {
    pinned: Option<Coordinates>,
    cache: Arc<Mutex<Option<(Coordinates, Instant)>>>,
}

impl LocationResolver {
    pub fn pinned(lat: f64, lon: f64) -> Self {
        Self {
            pinned: Some(Coordinates::new(lat, lon)),
            cache: Arc::new(Mutex::new(None)),
        }
    }

    pub fn auto() -> Self {
        Self {
            pinned: None,
            cache: Arc::new(Mutex::new(None)),
        }
    }

    pub fn resolve(&self) -> Result<Coordinates> {
        if let Some(coords) = self.pinned {
            return Ok(coords);
        }

        if let Ok(cache) = self.cache.lock() {
            if let Some((coords, at)) = *cache {
                if at.elapsed() < CACHE_TTL {
                    return Ok(coords);
                }
            }
        }

        let fix = corelocation::get_location().map_err(|e| {
            WeatherError::LocationFailed(e.to_string())
        })?;

        let coords = fix.coordinates;
        if let Ok(mut cache) = self.cache.lock() {
            *cache = Some((coords, Instant::now()));
        }

        Ok(coords)
    }

    pub fn clear_cache(&self) {
        if let Ok(mut cache) = self.cache.lock() {
            *cache = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pinned_coordinates_skip_corelocation() {
        let resolver = LocationResolver::pinned(52.52, 13.405);
        let coords = resolver.resolve().unwrap();
        assert!((coords.latitude - 52.52).abs() < f64::EPSILON);
        assert!((coords.longitude - 13.405).abs() < f64::EPSILON);
    }
}
