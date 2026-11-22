use alloc::format;
use alloc::vec::Vec;

use calor::{Place, Reading, Sensor};

pub const BASE: usize = 0xFE80_2000;
pub const SIZE: usize = 0x200;
const SENSORS: usize = 0x20;
const COUNT: usize = 12;
const CRITICAL: [(usize, usize); 2] = [(0, 0x55), (1, 0x57)];
const FAN: usize = 0x95;
const VALID: core::ops::RangeInclusive<u8> = 0x11..=0x7e;
const SANE_CRITICAL: core::ops::Range<u8> = 0x1e..0x80;
const DEFAULT_CRITICAL: u8 = 0x78;

pub fn reading(window: &[u8], time: u64) -> Option<Reading> {
    let mut sensors = Vec::new();
    for index in 0..COUNT {
        let raw = *window.get(SENSORS + index)?;
        if !VALID.contains(&raw) {
            continue;
        }
        let critical = CRITICAL
            .iter()
            .find(|(sensor, _)| *sensor == index)
            .and_then(|(_, at)| window.get(*at))
            .map(|limit| if SANE_CRITICAL.contains(limit) { *limit } else { DEFAULT_CRITICAL });
        sensors.push(Sensor {
            place: if index == 0 { Place::Processor } else { Place::Board },
            label: format!("EC{index}"),
            millis: i32::from(raw) * 1000,
            critical: critical.map(|limit| i32::from(limit) * 1000),
        });
    }
    let fan = window.get(FAN).map(|level| *level != 0);
    (!sensors.is_empty()).then_some(Reading { time, sensors, power: None, fan })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn window(temperatures: &[(usize, u8)], critical: (u8, u8), fan: u8) -> [u8; SIZE] {
        let mut window = [0u8; SIZE];
        for (index, celsius) in temperatures {
            window[SENSORS + index] = *celsius;
        }
        (window[0x55], window[0x57], window[FAN]) = (critical.0, critical.1, fan);
        window
    }

    #[test]
    fn the_ec_window_reads_as_the_acpi_thermal_zones_of_this_laptop() {
        let reading = reading(&window(&[(0, 43), (1, 42), (2, 0), (3, 0xff)], (118, 126), 0x32), 7).unwrap();
        assert_eq!(reading.sensors.len(), 2);
        let (first, second) = (&reading.sensors[0], &reading.sensors[1]);
        assert_eq!((first.place, first.label.as_str(), first.millis, first.critical), (Place::Processor, "EC0", 43_000, Some(118_000)));
        assert_eq!((second.place, second.millis, second.critical), (Place::Board, 42_000, Some(126_000)));
        assert_eq!((reading.time, reading.fan, reading.power), (7, Some(true), None));
        let defaulted = super::reading(&window(&[(0, 50)], (0x10, 0x90), 0), 1).unwrap();
        assert_eq!((defaulted.sensors[0].critical, defaulted.fan), (Some(120_000), Some(false)));
        assert!(super::reading(&window(&[], (118, 126), 0), 1).is_none());
        assert!(super::reading(&[0u8; 8], 1).is_none());
    }
}
