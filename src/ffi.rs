//! C FFI exports for WeatherKit.
//!
//! All functions are blocking network calls - call them from a worker
//! thread.

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_double};

use serde_json::{json, Value};

fn set_error(error_out: *mut *mut c_char, message: &str) {
    if error_out.is_null() {
        return;
    }
    if let Ok(c) = CString::new(message.to_owned()) {
        unsafe { *error_out = c.into_raw() };
    }
}

fn json_ptr(value: &Value) -> *mut c_char {
    CString::new(value.to_string())
        .unwrap_or_default()
        .into_raw()
}

fn weather_json(weather: &crate::types::CurrentWeather) -> Value {
    json!({
        "temperature_c": weather.temperature_c,
        "feels_like_c": weather.feels_like_c,
        "humidity_pct": weather.humidity_pct,
        "pressure_hpa": weather.pressure_hpa,
        "wind_kmh": weather.wind_kmh,
        "wind_direction_deg": weather.wind_direction_deg,
        "wind_gusts_kmh": weather.wind_gusts_kmh,
        "precipitation_mm": weather.precipitation_mm,
        "cloud_cover_pct": weather.cloud_cover_pct,
        "visibility_m": weather.visibility_m,
        "uv_index": weather.uv_index,
        "is_day": weather.is_day,
        "weather_code": weather.weather_code,
        "condition": weather.condition,
        "source": weather.source,
    })
}

fn forecast_json(day: &crate::types::ForecastDay) -> Value {
    json!({
        "date": day.date,
        "temp_max_c": day.temp_max_c,
        "temp_min_c": day.temp_min_c,
        "precipitation_mm": day.precipitation_mm,
        "precip_probability_pct": day.precip_probability_pct,
        "wind_max_kmh": day.wind_max_kmh,
        "weather_code": day.weather_code,
        "sunrise_utc": day.sunrise_utc,
        "sunset_utc": day.sunset_utc,
    })
}

/// The framework version as a static C string.
#[no_mangle]
pub extern "C" fn tontoo_weatherkit_version() -> *const c_char {
    concat!(env!("CARGO_PKG_VERSION"), "\0").as_ptr() as *const c_char
}

/// Current weather. Returns a JSON object or null.
///
/// # Safety
///
/// `error_out`, when not null, must point to a writable `char*`.
#[no_mangle]
pub unsafe extern "C" fn tontoo_weatherkit_current_weather(
    error_out: *mut *mut c_char,
) -> *mut c_char {
    match crate::current_weather() {
        Ok(weather) => json_ptr(&weather_json(&weather)),
        Err(_) => {
            set_error(error_out, "weather unavailable");
            std::ptr::null_mut()
        }
    }
}

/// Current temperature in Celsius. Returns NaN on error (see `error_out`).
///
/// # Safety
///
/// `error_out`, when not null, must point to a writable `char*`.
#[no_mangle]
pub unsafe extern "C" fn tontoo_weatherkit_temperature(
    error_out: *mut *mut c_char,
) -> c_double {
    match crate::temperature() {
        Ok(temp) => temp,
        Err(_) => {
            set_error(error_out, "weather unavailable");
            f64::NAN
        }
    }
}

/// Weekly forecast. Returns a JSON array or null.
///
/// # Safety
///
/// `error_out`, when not null, must point to a writable `char*`.
#[no_mangle]
pub unsafe extern "C" fn tontoo_weatherkit_weekly_forecast(
    error_out: *mut *mut c_char,
) -> *mut c_char {
    match crate::weekly_forecast() {
        Ok(days) => json_ptr(&Value::Array(
            days.iter().map(forecast_json).collect(),
        )),
        Err(_) => {
            set_error(error_out, "forecast unavailable");
            std::ptr::null_mut()
        }
    }
}

/// Free a string returned by this library.
///
/// # Safety
///
/// `s` must be a pointer returned by this API or null.
#[no_mangle]
pub unsafe extern "C" fn tontoo_weatherkit_string_free(s: *mut c_char) {
    if !s.is_null() {
        drop(CString::from_raw(s));
    }
}
