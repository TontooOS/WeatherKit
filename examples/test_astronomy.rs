use weatherkit::{astronomy, WeatherKit};

fn main() {
    let kit = WeatherKit::new();

    match kit.sun_times(2026, 8, 24) {
        Ok(times) => {
            if times.polar_day {
                println!("Polar day - sun never sets");
            } else if times.polar_night {
                println!("Polar night - sun never rises");
            } else {
                println!(
                    "Sunrise {:02}:{:02} UTC, sunset {:02}:{:02} UTC, day length {}h{}m",
                    times.sunrise.0,
                    times.sunrise.1,
                    times.sunset.0,
                    times.sunset.1,
                    times.day_length_secs / 3600,
                    (times.day_length_secs % 3600) / 60
                );
            }
        }
        Err(e) => eprintln!("sun times: {}", e),
    }

    let moon = kit.moon_phase();
    println!(
        "Moon: {} ({:.0}% illuminated, age {:.1} days)",
        moon.name, moon.illumination_pct, moon.age_days
    );

    match kit.is_day() {
        Ok(day) => println!("Daylight right now: {}", day),
        Err(e) => eprintln!("is_day: {}", e),
    }

    match kit.weather_for_place("Berlin") {
        Ok(weather) => println!(
            "\nBerlin right now: {:.1}°C, {} ({})",
            weather.temperature_c, weather.condition, weather.source
        ),
        Err(e) => eprintln!("place weather: {}", e),
    }

    let _ = astronomy::now_julian_day();
}
