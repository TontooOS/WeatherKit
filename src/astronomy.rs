use crate::types::{MoonPhase, SunTimes};

const J2000: f64 = 2451545.0;
const SYNODIC_MONTH: f64 = 29.530588853;
const NEW_MOON_EPOCH_JD: f64 = 2451550.1;

fn deg(value: f64) -> f64 {
    value.to_radians()
}

fn normalize_deg(value: f64) -> f64 {
    let mut result = value % 360.0;
    if result < 0.0 {
        result += 360.0;
    }
    result
}

/// Days since 1970-01-01 to a Gregorian year/month/day (Hinnant's algorithm).
pub fn civil_from_days(days_since_epoch: i64) -> (i32, u32, u32) {
    let z = days_since_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = if m <= 2 { y + 1 } else { y };
    (year as i32, m, d)
}

/// Julian day at 00:00 UT of the given date.
pub fn julian_day(year: i32, month: u32, day: u32) -> f64 {
    let y = year as f64;
    let m = month as f64;
    let d = day as f64;
    367.0 * y - (7.0 * (y + (m + 9.0) / 12.0).floor() / 4.0).floor()
        + (275.0 * m / 9.0).floor()
        + d
        + 1721013.5
}

fn julian_to_hhmm(jd: f64) -> (u32, u32) {
    let frac = jd - jd.floor();
    let total_secs = (frac * 86400.0).round() as u64;
    let hours = (total_secs / 3600) % 24;
    let minutes = (total_secs % 3600) / 60;
    (hours as u32, minutes as u32)
}

/// Current julian day including the time of day.
pub fn now_julian_day() -> f64 {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);
    secs / 86400.0 + 2440587.5
}

/// Computes sunrise/sunset in UTC with the standard sunrise equation.
///
/// Handles polar day and night explicitly instead of returning NaN values.
pub fn sun_times(lat: f64, lon: f64, year: i32, month: u32, day: u32) -> SunTimes {
    let jd_midnight = julian_day(year, month, day);
    let n = jd_midnight - J2000;
    let j_prime = n - lon / 360.0;

    let mean_anomaly = normalize_deg(357.5291 + 0.98560028 * j_prime);
    let center =
        1.9148 * deg(mean_anomaly).sin() + 0.02 * deg(2.0 * mean_anomaly).sin()
            + 0.0003 * deg(3.0 * mean_anomaly).sin();
    let ecliptic_lon = normalize_deg(mean_anomaly + center + 180.0 + 102.9372);

    let j_transit =
        J2000 + j_prime + 0.0053 * deg(mean_anomaly).sin() - 0.0069 * deg(2.0 * ecliptic_lon).sin();

    let declination_sin = deg(ecliptic_lon).sin() * deg(23.44).sin();
    let declination = declination_sin.asin();

    let hour_arg = (deg(-0.83).sin() - lat.to_radians().sin() * declination.sin())
        / (lat.to_radians().cos() * declination.cos());

    if hour_arg < -1.0 {
        return SunTimes {
            sunrise: (0, 0),
            sunset: (23, 59),
            day_length_secs: 86340,
            polar_day: true,
            polar_night: false,
        };
    }
    if hour_arg > 1.0 {
        return SunTimes {
            sunrise: (12, 0),
            sunset: (12, 0),
            day_length_secs: 0,
            polar_day: false,
            polar_night: true,
        };
    }

    let hour_angle = hour_arg.acos();
    let half_day_fraction = hour_angle / std::f64::consts::TAU;
    let sunrise_jd = j_transit - half_day_fraction;
    let sunset_jd = j_transit + half_day_fraction;

    let sunrise = julian_to_hhmm(sunrise_jd);
    let sunset = julian_to_hhmm(sunset_jd);

    let rise_secs = sunrise.0 as u64 * 3600 + sunrise.1 as u64 * 60;
    let set_secs = sunset.0 as u64 * 3600 + sunset.1 as u64 * 60;
    let day_length_secs = set_secs.saturating_sub(rise_secs);

    SunTimes {
        sunrise,
        sunset,
        day_length_secs,
        polar_day: false,
        polar_night: false,
    }
}

/// Whether the sun is above the horizon right now at this location.
pub fn is_daytime(lat: f64, lon: f64) -> bool {
    let now = now_julian_day();
    let (year, month, day) = civil_from_days((now.floor() - 2440587.5) as i64);
    let times = sun_times(lat, lon, year, month, day);

    if times.polar_day {
        return true;
    }
    if times.polar_night {
        return false;
    }

    let frac = now - now.floor();
    let minutes_now = (frac * 1440.0) as u32;
    let rise_minutes = times.sunrise.0 * 60 + times.sunrise.1;
    let set_minutes = times.sunset.0 * 60 + times.sunset.1;

    minutes_now >= rise_minutes && minutes_now <= set_minutes
}

/// Computes the current moon age, illumination and phase name locally.
pub fn moon_phase() -> MoonPhase {
    let jd = now_julian_day();
    let age = (jd - NEW_MOON_EPOCH_JD).rem_euclid(SYNODIC_MONTH);
    let fraction = age / SYNODIC_MONTH;
    let illumination = (1.0 - (2.0 * std::f64::consts::PI * fraction).cos()) / 2.0 * 100.0;

    MoonPhase {
        age_days: age,
        phase_fraction: fraction,
        illumination_pct: illumination.clamp(0.0, 100.0),
        name: phase_name(fraction),
    }
}

fn phase_name(fraction: f64) -> &'static str {
    match fraction {
        f if f < 0.02 || f >= 0.98 => "New Moon",
        f if f < 0.22 => "Waxing Crescent",
        f if f < 0.28 => "First Quarter",
        f if f < 0.47 => "Waxing Gibbous",
        f if f < 0.53 => "Full Moon",
        f if f < 0.72 => "Waning Gibbous",
        f if f < 0.78 => "Last Quarter",
        _ => "Waning Crescent",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_epoch_days_to_civil_dates() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(1), (1970, 1, 2));
        assert_eq!(civil_from_days(-1), (1969, 12, 31));
        // 2026-08-24 is 20689 days after the epoch
        assert_eq!(civil_from_days(20689), (2026, 8, 24));
    }

    #[test]
    fn berlin_summer_solstice_is_long() {
        let times = sun_times(52.52, 13.405, 2026, 6, 21);
        assert!(!times.polar_day && !times.polar_night);
        assert!(
            (58_000..=62_000).contains(&times.day_length_secs),
            "unexpected day length {}s",
            times.day_length_secs
        );
        let (rise_h, _) = times.sunrise;
        assert!((1..=4).contains(&rise_h), "sunrise hour {} UTC", rise_h);
    }

    #[test]
    fn equator_equinox_is_twelve_hours() {
        let times = sun_times(0.0, 0.0, 2026, 3, 20);
        assert!(
            (42_000..=45_000).contains(&times.day_length_secs),
            "unexpected day length {}s",
            times.day_length_secs
        );
    }

    #[test]
    fn polar_regions_report_extremes() {
        let midsummer_tromso = sun_times(69.65, 18.96, 2026, 6, 21);
        assert!(midsummer_tromso.polar_day);

        let midwinter_tromso = sun_times(69.65, 18.96, 2026, 12, 21);
        assert!(midwinter_tromso.polar_night);
    }

    #[test]
    fn moon_values_are_in_range() {
        let moon = moon_phase();
        assert!((0.0..=SYNODIC_MONTH).contains(&moon.age_days));
        assert!((0.0..=1.0).contains(&moon.phase_fraction));
        assert!((0.0..=100.0).contains(&moon.illumination_pct));
        assert!(!moon.name.is_empty());
    }

    #[test]
    fn phase_names_map_bins() {
        assert_eq!(phase_name(0.0), "New Moon");
        assert_eq!(phase_name(0.1), "Waxing Crescent");
        assert_eq!(phase_name(0.25), "First Quarter");
        assert_eq!(phase_name(0.35), "Waxing Gibbous");
        assert_eq!(phase_name(0.5), "Full Moon");
        assert_eq!(phase_name(0.6), "Waning Gibbous");
        assert_eq!(phase_name(0.75), "Last Quarter");
        assert_eq!(phase_name(0.85), "Waning Crescent");
        assert_eq!(phase_name(0.99), "New Moon");
    }
}
