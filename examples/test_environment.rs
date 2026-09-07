use weatherkit::WeatherKit;

fn main() {
    let kit = WeatherKit::new();

    match kit.air_quality() {
        Ok(air) => {
            println!(
                "Air quality ({}): EU AQI {:?}, US AQI {:?}",
                air.source, air.european_aqi, air.us_aqi
            );
            println!(
                "  PM10 {:?}, PM2.5 {:?}, O3 {:?}, NO2 {:?}",
                air.pm10, air.pm2_5, air.ozone, air.nitrogen_dioxide
            );
        }
        Err(e) => eprintln!("air quality: {}", e),
    }

    match kit.pollen() {
        Ok(pollen) => {
            println!("\nPollen:");
            println!("  alder {:?}", pollen.alder);
            println!("  birch {:?}", pollen.birch);
            println!("  grass {:?}", pollen.grass);
        }
        Err(e) => eprintln!("pollen: {}", e),
    }

    match kit.marine_conditions() {
        Ok(marine) => println!(
            "\nMarine ({}): waves {:?} m",
            marine.source, marine.wave_height_m
        ),
        Err(e) => println!("\nmarine: {} (inland location)", e),
    }

    match kit.historical_weather("2026-07-01", "2026-07-05") {
        Ok(days) => {
            println!("\nHistorical 2026-07-01..05:");
            for day in &days {
                println!(
                    "  {}: {:.1}/{:.1}°C, {:.1}mm",
                    day.date,
                    day.temp_min_c.unwrap_or(f64::NAN),
                    day.temp_max_c.unwrap_or(f64::NAN),
                    day.precipitation_mm.unwrap_or(0.0)
                );
            }
        }
        Err(e) => eprintln!("historical: {}", e),
    }
}
