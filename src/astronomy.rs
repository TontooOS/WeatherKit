use crate::types::{MoonPhase, MoonTimes, SunTimes, TwilightTimes};

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
    match sun_event(lat, lon, year, month, day, 90.833) {
        Some((rise, set, length)) => SunTimes {
            sunrise: rise,
            sunset: set,
            day_length_secs: length,
            polar_day: false,
            polar_night: false,
        },
        None if polar_day(lat, lon, year, month, day) => SunTimes {
            sunrise: (0, 0),
            sunset: (23, 59),
            day_length_secs: 86340,
            polar_day: true,
            polar_night: false,
        },
        None => SunTimes {
            sunrise: (12, 0),
            sunset: (12, 0),
            day_length_secs: 0,
            polar_day: false,
            polar_night: true,
        },
    }
}

/// Generic sunrise-equation solver for any zenith angle.
///
/// Returns `None` when the sun never crosses the zenith that day (polar day
/// or polar night for that angle).
fn sun_event(
    lat: f64,
    lon: f64,
    year: i32,
    month: u32,
    day: u32,
    zenith_deg: f64,
) -> Option<((u32, u32), (u32, u32), u64)> {
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

    let hour_arg = hour_argument(lat, declination, zenith_deg);
    if hour_arg < -1.0 || hour_arg > 1.0 {
        return None;
    }

    let hour_angle = hour_arg.acos();
    let half_day_fraction = hour_angle / std::f64::consts::TAU;
    let rise_jd = j_transit - half_day_fraction;
    let set_jd = j_transit + half_day_fraction;

    let sunrise = julian_to_hhmm(rise_jd);
    let sunset = julian_to_hhmm(set_jd);

    let rise_secs = sunrise.0 as u64 * 3600 + sunrise.1 as u64 * 60;
    let set_secs = sunset.0 as u64 * 3600 + sunset.1 as u64 * 60;
    let day_length_secs = set_secs.saturating_sub(rise_secs);

    Some((sunrise, sunset, day_length_secs))
}

fn hour_argument(lat: f64, declination: f64, zenith_deg: f64) -> f64 {
    (zenith_deg.to_radians().cos() - lat.to_radians().sin() * declination.sin())
        / (lat.to_radians().cos() * declination.cos())
}

/// Whether the sun stays up all day (used to separate polar day/night).
fn polar_day(lat: f64, lon: f64, year: i32, month: u32, day: u32) -> bool {
    let jd_midnight = julian_day(year, month, day);
    let n = jd_midnight - J2000 + 0.5 - lon / 360.0;
    let mean_anomaly = normalize_deg(357.5291 + 0.98560028 * n);
    let center =
        1.9148 * deg(mean_anomaly).sin() + 0.02 * deg(2.0 * mean_anomaly).sin()
            + 0.0003 * deg(3.0 * mean_anomaly).sin();
    let ecliptic_lon = normalize_deg(mean_anomaly + center + 180.0 + 102.9372);
    let declination = (deg(ecliptic_lon).sin() * deg(23.44).sin()).asin();
    lat.to_radians().sin() * declination.sin()
        + lat.to_radians().cos() * declination.cos()
        > deg(-0.83).sin()
}

/// Civil (6 deg), nautical (12 deg) and astronomical (18 deg) twilight.
///
/// Fully offline like [`sun_times`]. Entries are `None` when the sun never
/// reaches that depression angle (polar summer/winter).
pub fn twilight_times(lat: f64, lon: f64, year: i32, month: u32, day: u32) -> TwilightTimes {
    let pair = |zenith: f64| -> (Option<(u32, u32)>, Option<(u32, u32)>) {
        match sun_event(lat, lon, year, month, day, zenith) {
            Some((rise, set, _)) => (Some(rise), Some(set)),
            None => (None, None),
        }
    };
    let (dawn_civil, dusk_civil) = pair(96.0);
    let (dawn_nautical, dusk_nautical) = pair(102.0);
    let (dawn_astronomical, dusk_astronomical) = pair(108.0);
    TwilightTimes {
        dawn_civil,
        dusk_civil,
        dawn_nautical,
        dusk_nautical,
        dawn_astronomical,
        dusk_astronomical,
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

fn gmst_hours(jd: f64) -> f64 {
    normalize_deg(280.46061837 + 360.98564736629 * (jd - 2451545.0)) / 15.0
}

/// Low-precision lunar equatorial coordinates (Paul Schlyter algorithm).
///
/// Accurate to a few arcminutes, which keeps rise/set times within roughly
/// fifteen minutes. Fully offline, no network involved.
fn moon_equatorial(jd: f64) -> (f64, f64) {
    let d = jd - 2451543.5;
    let node = normalize_deg(125.1228 - 0.0529538083 * d);
    let incl = 5.1454_f64.to_radians();
    let peri = normalize_deg(318.0634 + 0.1643573223 * d);
    let ecc = 0.054900;
    let anomaly = normalize_deg(115.3654 + 13.0649929509 * d);

    let m_rad = anomaly.to_radians();
    let mut eccentric = m_rad + ecc * m_rad.sin() * (1.0 + ecc * m_rad.cos());
    for _ in 0..3 {
        eccentric -= (eccentric - ecc * eccentric.sin() - m_rad) / (1.0 - ecc * eccentric.cos());
    }
    let xv = 60.2666 * (eccentric.cos() - ecc);
    let yv = 60.2666 * ((1.0 - ecc * ecc).sqrt() * eccentric.sin());
    let dist = (xv * xv + yv * yv).sqrt();
    let true_anomaly = yv.atan2(xv).to_degrees();

    let lon_orbit = normalize_deg(true_anomaly + peri);
    let node_rad = node.to_radians();
    let lon_rad = lon_orbit.to_radians();
    let xh = dist * (node_rad.cos() * lon_rad.cos());
    let yh = dist * (node_rad.sin() * lon_rad.cos() * incl.cos() - lon_rad.sin() * incl.sin());
    let zh = dist * (node_rad.sin() * lon_rad.cos() * incl.sin() + lon_rad.sin() * incl.cos());
    let mut lon_ecl = normalize_deg(yh.atan2(xh).to_degrees());
    let mut lat_ecl = (zh / dist).asin().to_degrees();

    let sun_anomaly = normalize_deg(356.0470 + 0.9856002585 * d);
    let sun_lon = normalize_deg(sun_anomaly + 282.9404 + 4.70935e-5 * d);
    let mean_lon = normalize_deg(node + peri + anomaly);
    let elong = normalize_deg(mean_lon - sun_lon);
    let arg_lat = normalize_deg(mean_lon - node);

    let d_rad = elong.to_radians();
    let m_rad = anomaly.to_radians();
    let ms_rad = sun_anomaly.to_radians();
    let f_rad = arg_lat.to_radians();
    lon_ecl += -1.274 * (m_rad - 2.0 * d_rad).sin()
        + 0.658 * (2.0 * d_rad).sin()
        - 0.186 * ms_rad.sin()
        - 0.059 * (2.0 * m_rad - 2.0 * d_rad).sin()
        - 0.057 * (m_rad - 2.0 * d_rad + ms_rad).sin();
    lat_ecl += -0.173 * (f_rad - 2.0 * d_rad).sin()
        - 0.055 * (m_rad - f_rad - 2.0 * d_rad).sin()
        - 0.046 * (m_rad + f_rad - 2.0 * d_rad).sin()
        + 0.033 * (f_rad + 2.0 * d_rad).sin();

    let obliquity = (23.4393 - 3.563e-7 * d).to_radians();
    let lon_rad = lon_ecl.to_radians();
    let lat_rad = lat_ecl.to_radians();
    let x = lon_rad.cos() * lat_rad.cos();
    let y = lon_rad.sin() * lat_rad.cos() * obliquity.cos() - lat_rad.sin() * obliquity.sin();
    let z = lon_rad.sin() * lat_rad.cos() * obliquity.sin() + lat_rad.sin() * obliquity.cos();
    let ra = normalize_deg(y.atan2(x).to_degrees()) / 15.0;
    let dec = z.asin().to_degrees();
    (ra, dec)
}

fn moon_altitude_deg(lat: f64, lon: f64, jd: f64) -> f64 {
    let (ra_hours, dec_deg) = moon_equatorial(jd);
    let lst = gmst_hours(jd) + lon / 15.0;
    let hour_angle = ((lst - ra_hours) * 15.0).to_radians();
    let lat_rad = lat.to_radians();
    let dec_rad = dec_deg.to_radians();
    (dec_rad.sin() * lat_rad.sin() + dec_rad.cos() * lat_rad.cos() * hour_angle.cos()).asin()
        .to_degrees()
}

/// Moonrise and moonset in UTC for the given civil date.
///
/// Samples the low-precision lunar altitude every ten minutes and
/// interpolates horizon crossings. Accuracy is roughly fifteen minutes;
/// entries are `None` when the moon stays up or down all day. Fully offline.
pub fn moon_times(lat: f64, lon: f64, year: i32, month: u32, day: u32) -> MoonTimes {
    let jd0 = julian_day(year, month, day);
    let step_days = 10.0 / 1440.0;
    let steps = 144;

    let mut altitudes = Vec::with_capacity(steps + 1);
    for index in 0..=steps {
        altitudes.push(moon_altitude_deg(lat, lon, jd0 + index as f64 * step_days));
    }

    let mut moonrise = None;
    let mut moonset = None;
    for window in altitudes.windows(2).enumerate() {
        let (prev, next) = (window.1[0], window.1[1]);
        if prev < 0.0 && next >= 0.0 && moonrise.is_none() {
            let frac = prev.abs() / (next - prev);
            moonrise = Some(julian_to_hhmm(jd0 + (window.0 as f64 + frac) * step_days));
        } else if prev >= 0.0 && next < 0.0 && moonset.is_none() {
            let frac = prev / (prev - next);
            moonset = Some(julian_to_hhmm(jd0 + (window.0 as f64 + frac) * step_days));
        }
        if moonrise.is_some() && moonset.is_some() {
            break;
        }
    }

    let phase = moon_phase();
    MoonTimes {
        moonrise,
        moonset,
        illumination_pct: phase.illumination_pct,
        phase_name: phase.name,
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

    #[test]
    fn twilight_ordering_equator() {
        // At the equator on equinox every twilight occurs, so full ordering holds.
        let tw = twilight_times(0.0, 0.0, 2026, 3, 20);
        let to_min = |t: Option<(u32, u32)>| t.map(|(h, m)| h * 60 + m).unwrap();
        assert!(to_min(tw.dawn_astronomical) < to_min(tw.dawn_nautical));
        assert!(to_min(tw.dawn_nautical) < to_min(tw.dawn_civil));
        assert!(to_min(tw.dusk_civil) < to_min(tw.dusk_nautical));
        assert!(to_min(tw.dusk_nautical) < to_min(tw.dusk_astronomical));
    }

    #[test]
    fn twilight_summer_night_never_dark() {
        // Berlin in June never reaches astronomical night; missing entries
        // are None instead of wrong times.
        let tw = twilight_times(52.52, 13.405, 2026, 6, 21);
        assert!(tw.dawn_civil.is_some() && tw.dusk_civil.is_some());
        assert!(tw.dawn_astronomical.is_none() || tw.dusk_astronomical.is_none());
    }

    #[test]
    fn polar_night_has_no_twilight_events() {
        let tw = twilight_times(78.22, 15.63, 2026, 12, 21);
        assert!(tw.dawn_civil.is_none() || tw.dusk_civil.is_none());
    }

    #[test]
    fn moon_times_in_valid_range() {
        let moon = moon_times(52.52, 13.405, 2026, 8, 24);
        for event in [moon.moonrise, moon.moonset].into_iter().flatten() {
            assert!(event.0 < 24 && event.1 < 60);
        }
        assert!((0.0..=100.0).contains(&moon.illumination_pct));
        assert!(!moon.phase_name.is_empty());
    }

    #[test]
    fn moon_equatorial_in_range() {
        let (ra, dec) = moon_equatorial(now_julian_day());
        assert!((0.0..24.0).contains(&ra));
        assert!((-30.0..30.0).contains(&dec));
    }
}
