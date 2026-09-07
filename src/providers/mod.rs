pub mod air_quality;
pub mod geocode;
pub mod met_no;
pub mod open_meteo;
pub mod wttr;

use crate::types::{AirQuality, MarineConditions, Place};

/// Air quality always has two domains behind it.
pub fn air_quality(lat: f64, lon: f64) -> crate::types::Result<AirQuality> {
    air_quality::fetch(lat, lon)
}

/// Marine conditions try ECMWF waves first, then MET Norway oceans.
pub fn marine(lat: f64, lon: f64) -> crate::types::Result<MarineConditions> {
    open_meteo::fetch_marine(lat, lon).or_else(|_| met_no::fetch_marine(lat, lon))
}

/// Places resolve through Open-Meteo geocoding with Nominatim as backup.
pub fn places(query: &str, limit: usize) -> crate::types::Result<Vec<Place>> {
    geocode::search(query, limit)
}
