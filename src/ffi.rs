//! C FFI exports for WeatherKit.
//!
//! All functions are blocking network calls - call them from a worker
//! thread.

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_double};

use foundation::serialization::JsonValue;

fn set_error(error_out: *mut *mut c_char, message: &str) {
    if error_out.is_null() {
        return;
    }
    if let Ok(c) = CString::new(message.to_owned()) {
        unsafe { *error_out = c.into_raw() };
    }
}

fn json_ptr(json: &str) -> *mut c_char {
    CString::new(json).unwrap_or_default().into_raw()
}

fn opt_i32(value: Option<i32>) -> JsonValue {
    match value {
        Some(v) => JsonValue::Integer(v as i64),
        None => JsonValue::Null,
    }
}

fn opt_f64(value: Option<f64>) -> JsonValue {
    match value {
        Some(v) => JsonValue::Float(v),
        None => JsonValue::Null,
    }
}

fn opt_string(value: &Option<String>) -> JsonValue {
    match value {
        Some(text) => JsonValue::Str(text.clone()),
        None => JsonValue::Null,
    }
}

fn weather_json(weather: &crate::types::CurrentWeather) -> String {
    JsonValue::Object(vec![
        ("temperature_c".to_string(), JsonValue::Float(weather.temperature_c)),
        ("feels_like_c".to_string(), JsonValue::Float(weather.feels_like_c)),
        ("humidity_pct".to_string(), JsonValue::Integer(weather.humidity_pct as i64)),
        ("pressure_hpa".to_string(), JsonValue::Float(weather.pressure_hpa)),
        ("wind_kmh".to_string(), JsonValue::Float(weather.wind_kmh)),
        (
            "wind_direction_deg".to_string(),
            JsonValue::Integer(weather.wind_direction_deg as i64),
        ),
        ("wind_gusts_kmh".to_string(), opt_f64(weather.wind_gusts_kmh)),
        ("precipitation_mm".to_string(), JsonValue::Float(weather.precipitation_mm)),
        ("cloud_cover_pct".to_string(), opt_i32(weather.cloud_cover_pct)),
        ("visibility_m".to_string(), opt_f64(weather.visibility_m)),
        ("uv_index".to_string(), opt_f64(weather.uv_index)),
        ("is_day".to_string(), JsonValue::Bool(weather.is_day)),
        ("weather_code".to_string(), opt_i32(weather.weather_code)),
        ("condition".to_string(), JsonValue::Str(weather.condition.clone())),
        ("source".to_string(), JsonValue::Str(weather.source.clone())),
    ])
    .stringify(false)
}

fn forecast_json(day: &crate::types::ForecastDay) -> String {
    JsonValue::Object(vec![
        ("date".to_string(), JsonValue::Str(day.date.clone())),
        ("temp_max_c".to_string(), JsonValue::Float(day.temp_max_c)),
        ("temp_min_c".to_string(), JsonValue::Float(day.temp_min_c)),
        ("precipitation_mm".to_string(), opt_f64(day.precipitation_mm)),
        (
            "precip_probability_pct".to_string(),
            opt_i32(day.precip_probability_pct),
        ),
        ("wind_max_kmh".to_string(), JsonValue::Float(day.wind_max_kmh)),
        ("weather_code".to_string(), opt_i32(day.weather_code)),
        ("sunrise_utc".to_string(), opt_string(&day.sunrise_utc)),
        ("sunset_utc".to_string(), opt_string(&day.sunset_utc)),
    ])
    .stringify(false)
}

fn alert_json(alert: &crate::types::WeatherAlert) -> String {
    JsonValue::Object(vec![
        ("headline".to_string(), JsonValue::Str(alert.headline.clone())),
        ("description".to_string(), JsonValue::Str(alert.description.clone())),
        ("severity".to_string(), JsonValue::Str(alert.severity.as_str().to_string())),
        ("kind".to_string(), JsonValue::Str(alert.kind.clone())),
        ("source".to_string(), JsonValue::Str(alert.source.clone())),
        ("effective".to_string(), opt_string(&alert.effective)),
        ("expires".to_string(), opt_string(&alert.expires)),
        ("should_notify".to_string(), JsonValue::Bool(alert.should_notify())),
    ])
    .stringify(false)
}

fn minutely_json(point: &crate::types::MinutePoint) -> String {
    JsonValue::Object(vec![
        ("time".to_string(), JsonValue::Str(point.time.clone())),
        ("precipitation_mm".to_string(), JsonValue::Float(point.precipitation_mm)),
        (
            "precip_probability_pct".to_string(),
            opt_i32(point.precip_probability_pct),
        ),
        ("temperature_c".to_string(), opt_f64(point.temperature_c)),
    ])
    .stringify(false)
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
        Ok(days) => {
            let items: Vec<String> = days.iter().map(forecast_json).collect();
            json_ptr(&format!("[{}]", items.join(",")))
        }
        Err(_) => {
            set_error(error_out, "forecast unavailable");
            std::ptr::null_mut()
        }
    }
}

/// Active weather alerts. Returns a JSON array or null.
///
/// # Safety
///
/// `error_out`, when not null, must point to a writable `char*`.
#[no_mangle]
pub unsafe extern "C" fn tontoo_weatherkit_active_alerts(
    error_out: *mut *mut c_char,
) -> *mut c_char {
    match crate::WeatherKit::new().active_alerts() {
        Ok(alerts) => {
            let items: Vec<String> = alerts.iter().map(alert_json).collect();
            json_ptr(&format!("[{}]", items.join(",")))
        }
        Err(_) => {
            set_error(error_out, "alerts unavailable");
            std::ptr::null_mut()
        }
    }
}

/// Next-hour precipitation in 15 minute steps. Returns a JSON array or null.
///
/// # Safety
///
/// `error_out`, when not null, must point to a writable `char*`.
#[no_mangle]
pub unsafe extern "C" fn tontoo_weatherkit_minutely_precipitation(
    error_out: *mut *mut c_char,
) -> *mut c_char {
    match crate::WeatherKit::new().minutely_precipitation(60) {
        Ok(points) => {
            let items: Vec<String> = points.iter().map(minutely_json).collect();
            json_ptr(&format!("[{}]", items.join(",")))
        }
        Err(_) => {
            set_error(error_out, "minutely forecast unavailable");
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
