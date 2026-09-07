use weatherkit::WeatherKit;

fn main() {
    let kit = WeatherKit::new();

    let current = match kit.current_weather() {
        Ok(current) => current,
        Err(e) => {
            eprintln!("current: {}", e);
            return;
        }
    };

    println!(
        "Current ({}): {:.1}°C, feels like {:.1}°C, {}",
        current.source, current.temperature_c, current.feels_like_c, current.condition
    );
    println!(
        "Humidity {}%, pressure {:.0} hPa, wind {:.1} km/h from {}°",
        current.humidity_pct, current.pressure_hpa, current.wind_kmh, current.wind_direction_deg
    );

    println!("\nIndividual getters:");
    println!("  temperature():      {:?}", kit.temperature());
    println!("  feels_like():       {:?}", kit.feels_like());
    println!("  humidity():         {:?}", kit.humidity());
    println!("  pressure():         {:?}", kit.pressure());
    println!("  wind_speed():       {:?}", kit.wind_speed());
    println!("  wind_direction():   {:?}", kit.wind_direction());
    println!("  wind_gusts():       {:?}", kit.wind_gusts());
    println!("  uv_index():         {:?}", kit.uv_index());
    println!("  visibility():       {:?}", kit.visibility());
    println!("  precipitation_now(): {:?}", kit.precipitation_now());
    println!("  is_day():           {:?}", kit.is_day());
}
