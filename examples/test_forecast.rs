use weatherkit::WeatherKit;

fn main() {
    let kit = WeatherKit::new();

    match kit.daily_forecast(7) {
        Ok(days) => {
            println!("Daily forecast:");
            for day in &days {
                println!(
                    "  {}: {:.0}/{:.0}°C, {:.1}mm ({:?}%), wind {:.0} km/h",
                    day.date,
                    day.temp_min_c,
                    day.temp_max_c,
                    day.precipitation_mm.unwrap_or(0.0),
                    day.precip_probability_pct,
                    day.wind_max_kmh
                );
            }
        }
        Err(e) => eprintln!("daily forecast: {}", e),
    }

    println!();
    match kit.hourly_forecast(12) {
        Ok(hours) => {
            println!("Next 12 hours:");
            for hour in &hours {
                println!(
                    "  {}: {:.1}°C, {:.1}mm, {:.0} km/h",
                    hour.time, hour.temperature_c, hour.precipitation_mm, hour.wind_kmh
                );
            }
        }
        Err(e) => eprintln!("hourly forecast: {}", e),
    }
}
