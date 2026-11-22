use std::path::{Path, PathBuf};
use std::time::SystemTime;

use calor::{Place, Reading, Sensor};

pub const LOCALES: [(&str, &str); 2] = [("ru", include_str!("../../locales/ru.ftl")), ("en", include_str!("../../locales/en.ftl"))];
pub const HWMON: &str = "/sys/class/hwmon";
pub const THERMAL: &str = "/sys/class/thermal";
const SECONDS_PER_DAY: u64 = 86_400;

pub fn catalog() -> drv_i18n::Catalog {
    drv_i18n::Catalog::new(&LOCALES)
}

fn text(path: &Path) -> Option<String> {
    std::fs::read_to_string(path).ok().map(|value| value.trim().to_string())
}

fn number(path: &Path) -> Option<i64> {
    text(path)?.parse().ok()
}

fn entries(root: &Path, prefix: &str) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(root)
        .map(|entries| {
            entries
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| path.file_name().and_then(|name| name.to_str()).is_some_and(|name| name.starts_with(prefix)))
                .collect()
        })
        .unwrap_or_default();
    found.sort();
    found
}

fn place(name: &str) -> Option<Place> {
    match name {
        "k10temp" | "zenpower" | "coretemp" => Some(Place::Processor),
        "amdgpu" => Some(Place::Graphics),
        "nvme" => Some(Place::Storage),
        name if name.starts_with("mt79") || name.starts_with("iwlwifi") || name.starts_with("ath") => Some(Place::Wireless),
        _ => None,
    }
}

fn millis(value: i64) -> Option<i32> {
    i32::try_from(value).ok()
}

pub fn reading(hwmon: &Path, thermal: &Path, time: u64) -> Reading {
    let mut sensors = Vec::new();
    let mut power = None;
    for dir in entries(hwmon, "hwmon") {
        let Some(name) = text(&dir.join("name")) else { continue };
        if name == "amdgpu" {
            power = number(&dir.join("power1_input")).or_else(|| number(&dir.join("power1_average"))).and_then(|micro| u32::try_from(micro / 1000).ok());
        }
        let (Some(place), Some(value)) = (place(&name), number(&dir.join("temp1_input")).and_then(millis)) else { continue };
        let label = text(&dir.join("temp1_label")).map_or_else(|| name.clone(), |label| format!("{name} {label}"));
        sensors.push(Sensor { place, label, millis: value, critical: number(&dir.join("temp1_crit")).and_then(millis) });
    }
    for zone in entries(thermal, "thermal_zone") {
        if text(&zone.join("type")).as_deref() != Some("acpitz") {
            continue;
        }
        let Some(value) = number(&zone.join("temp")).and_then(millis) else { continue };
        let critical = (0..8).find_map(|trip| {
            (text(&zone.join(format!("trip_point_{trip}_type"))).as_deref() == Some("critical"))
                .then(|| number(&zone.join(format!("trip_point_{trip}_temp"))).and_then(millis))
                .flatten()
        });
        let label = zone.file_name().and_then(|name| name.to_str()).unwrap_or("zone").replace("thermal_", "acpitz ");
        sensors.push(Sensor { place: Place::Board, label, millis: value, critical });
    }
    let fans: Vec<i64> = entries(thermal, "cooling_device")
        .into_iter()
        .filter(|device| text(&device.join("type")).as_deref() == Some("Fan"))
        .filter_map(|device| number(&device.join("cur_state")))
        .collect();
    let fan = (!fans.is_empty()).then(|| fans.iter().any(|state| *state > 0));
    Reading { time, sensors, power, fan }
}

pub fn now() -> u64 {
    let mut at = libc::timespec { tv_sec: 0, tv_nsec: 0 };
    unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut at) };
    u64::try_from(at.tv_sec).unwrap_or(0).saturating_mul(1_000_000).saturating_add(u64::try_from(at.tv_nsec).unwrap_or(0) / 1000)
}

pub fn today() -> u32 {
    let seconds = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).map_or(0, |since| since.as_secs());
    u32::try_from(seconds / SECONDS_PER_DAY).unwrap_or(u32::MAX)
}

pub fn diary() -> Option<PathBuf> {
    let state = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| Some(PathBuf::from(std::env::var_os("HOME")?).join(".local/state")))?;
    Some(state.join("lenthm").join("cooling"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn put(dir: &Path, files: &[(&str, &str)]) {
        std::fs::create_dir_all(dir).unwrap();
        for (file, value) in files {
            std::fs::write(dir.join(file), format!("{value}\n")).unwrap();
        }
    }

    #[test]
    fn this_laptops_sensors_read_as_one_reading() {
        let root = std::env::temp_dir().join(format!("lenthm-sysfs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let (hwmon, thermal) = (root.join("hwmon"), root.join("thermal"));
        put(&hwmon.join("hwmon1"), &[("name", "acpitz"), ("temp1_input", "43000")]);
        put(&hwmon.join("hwmon3"), &[("name", "nvme"), ("temp1_input", "36850"), ("temp1_label", "Composite"), ("temp1_crit", "84850")]);
        put(&hwmon.join("hwmon4"), &[("name", "amdgpu"), ("temp1_input", "43000"), ("temp1_label", "edge"), ("power1_input", "6000000")]);
        put(&hwmon.join("hwmon5"), &[("name", "k10temp"), ("temp1_input", "43375"), ("temp1_label", "Tctl")]);
        put(&hwmon.join("hwmon8"), &[("name", "mt7921_phy0"), ("temp1_input", "44000")]);
        put(&hwmon.join("hwmon2"), &[("name", "BAT0"), ("power1_input", "0")]);
        put(&thermal.join("thermal_zone0"), &[("type", "acpitz"), ("temp", "43000"), ("trip_point_0_type", "critical"), ("trip_point_0_temp", "118000")]);
        put(&thermal.join("thermal_zone1"), &[("type", "acpitz"), ("temp", "42000"), ("trip_point_0_type", "critical"), ("trip_point_0_temp", "126000")]);
        put(&thermal.join("cooling_device0"), &[("type", "Fan"), ("cur_state", "1")]);
        put(&thermal.join("cooling_device1"), &[("type", "Processor"), ("cur_state", "0")]);
        let reading = reading(&hwmon, &thermal, 5);
        let labels: Vec<(&str, Place, i32, Option<i32>)> =
            reading.sensors.iter().map(|sensor| (sensor.label.as_str(), sensor.place, sensor.millis, sensor.critical)).collect();
        assert_eq!(
            labels,
            [
                ("nvme Composite", Place::Storage, 36_850, Some(84_850)),
                ("amdgpu edge", Place::Graphics, 43_000, None),
                ("k10temp Tctl", Place::Processor, 43_375, None),
                ("mt7921_phy0", Place::Wireless, 44_000, None),
                ("acpitz zone0", Place::Board, 43_000, Some(118_000)),
                ("acpitz zone1", Place::Board, 42_000, Some(126_000)),
            ]
        );
        assert_eq!((reading.power, reading.fan, reading.time), (Some(6_000), Some(true), 5));
        assert_eq!(reading.processor().unwrap().label, "k10temp Tctl");
        let empty = super::reading(&root.join("none"), &root.join("none"), 1);
        assert!(empty.sensors.is_empty() && empty.fan.is_none() && empty.power.is_none());
        let _ = std::fs::remove_dir_all(&root);
    }
}
