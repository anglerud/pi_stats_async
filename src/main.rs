use std::collections::HashMap;
use std::fmt;
use std::io::{self, Write};

use futures::stream::StreamExt; // for `next` on streams.
use heim::{cpu, memory, sensors, units, Result};
use tokio::time::{sleep, Duration};

/// Hardware stats: cpu frequency, temperature and available RAM.
#[derive(Debug)]
struct PiStats {
    /// CPU frequency, core average presumably.
    cpu_frequency: units::Frequency,
    /// This is the composite, or 'CPU' temperature
    temperature: units::ThermodynamicTemperature,
    /// Available memory
    /// Note that this is different from 'free' memory in that this
    /// takes into account disk cache and buffers that the OS will
    /// reclaim under pressure.
    memory_available: units::Information,
}

impl fmt::Display for PiStats {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} Mhz / {} C / {} MiB",
            self.cpu_frequency.get::<units::frequency::megahertz>(),
            self.temperature
                .get::<units::thermodynamic_temperature::degree_celsius>(),
            self.memory_available.get::<units::information::mebibyte>()
        )
    }
}

/// Get the CPU temperature.
/// This guesses a little about which sensor is appropriate, we pick
/// Composite preferentially, and CPU which we know works on the
/// Raspberry Pi.
async fn cpu_temperature() -> units::ThermodynamicTemperature {
    // We stuff all the sensors into a hashmap, then pull out our
    // preferred sensors by label names.
    let composite_label = String::from("Composite");
    let cpu_label = String::from("CPU");
    let temp_default = units::ThermodynamicTemperature::new::<
        units::thermodynamic_temperature::degree_celsius,
    >(0.0);

    let mut temperature_sensors = HashMap::new();

    let mut sensors = sensors::temperatures();
    while let Some(Ok(sensor)) = sensors.next().await {
        temperature_sensors.insert(
            String::from(sensor.label().unwrap_or("unknown")),
            sensor.current(),
        );
    }

    *temperature_sensors
        .get(&composite_label)
        .ok_or(temperature_sensors.get(&cpu_label))
        .unwrap_or(&temp_default)
}

#[tokio::main]
async fn main() -> Result<()> {
    loop {
        let hardware_status = PiStats {
            cpu_frequency: cpu::frequency().await?.current(),
            temperature: cpu_temperature().await,
            memory_available: memory::memory().await?.available(),
        };

        // Clear line, print the hardware stats, return to start of line.
        print!("\x1b[2K"); // \x1b is escape code (27 in hex).
        print!("{}\r", hardware_status);
        io::stdout().flush().unwrap();

        sleep(Duration::from_secs(1)).await;
    }
}
