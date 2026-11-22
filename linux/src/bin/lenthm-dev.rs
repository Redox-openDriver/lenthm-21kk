use std::path::Path;
use std::process::ExitCode;
use std::time::Duration;

use calor::{Cooling, Notice, Place, Reading, Sensor, Watch};
use drv_i18n::{Catalog, Fail, Msg, as_msg};

type BoxError = Box<dyn std::error::Error>;

const EVERY: Duration = Duration::from_secs(2);
const WATCH_MINUTES: u64 = 5;
const LONGEST: u64 = 24 * 60;
const MICROS_PER_MINUTE: f32 = 60_000_000.0;
const SLOPE_WINDOW: u64 = 30_000_000;

fn place_key(place: Place) -> &'static str {
    match place {
        Place::Processor => "dev-place-processor",
        Place::Graphics => "dev-place-graphics",
        Place::Board => "dev-place-board",
        Place::Storage => "dev-place-storage",
        Place::Wireless => "dev-place-wireless",
        Place::Battery => "dev-place-battery",
        Place::Other => "dev-place-other",
    }
}

pub fn notice(said: &Notice) -> Msg {
    match said {
        Notice::Hot { label, celsius } => Msg::new("thermal-hot").arg("sensor", label).num("celsius", *celsius),
        Notice::Cooled { label, celsius } => Msg::new("thermal-cooled").arg("sensor", label).num("celsius", *celsius),
        Notice::NearCritical { label, celsius, critical } => {
            Msg::new("thermal-near-critical").arg("sensor", label).num("celsius", *celsius).num("critical", *critical)
        }
        Notice::CoolingWorse { percent } => Msg::new("thermal-cooling-worse").num("percent", *percent),
    }
}

fn read() -> Result<Reading, BoxError> {
    let reading = lenthm_linux::reading(Path::new(lenthm_linux::HWMON), Path::new(lenthm_linux::THERMAL), lenthm_linux::now());
    if reading.sensors.is_empty() {
        return Err(Fail::new("dev-no-sensors").into());
    }
    Ok(reading)
}

fn sensor_line(catalog: &Catalog, sensor: &Sensor) -> String {
    let message = Msg::new(if sensor.critical.is_some() { "dev-sensor-critical" } else { "dev-sensor" })
        .arg("place", catalog.text(place_key(sensor.place)))
        .arg("sensor", &sensor.label)
        .num("celsius", sensor.millis / 1000)
        .num("critical", sensor.critical.unwrap_or(0) / 1000);
    catalog.render(&message)
}

fn status(catalog: &Catalog) -> Result<(), BoxError> {
    let reading = read()?;
    let mut sensors = reading.sensors.clone();
    sensors.sort_by_key(|sensor| (sensor.place, sensor.label.clone()));
    for sensor in &sensors {
        println!("{}", sensor_line(catalog, sensor));
    }
    if let Some(power) = reading.power {
        println!("{}", catalog.render(&Msg::new("dev-power").arg("watts", format!("{:.1}", power as f32 / 1000.0))));
    }
    println!(
        "{}",
        catalog.text(match reading.fan {
            Some(true) => "dev-fan-on",
            Some(false) => "dev-fan-off",
            None => "dev-fan-unknown",
        })
    );
    let samples = lenthm_linux::diary().map(|path| lenthm_core::diary::samples(&path, lenthm_linux::today()).unwrap_or_default()).unwrap_or_default();
    let message = match calor::assess(&samples, lenthm_linux::today()) {
        None => Msg::new("dev-cooling-learning").num("samples", samples.len()),
        Some(change) if change.percent >= calor::watch::WORSE => Msg::new("dev-cooling-worse").num("percent", change.percent as i32),
        Some(change) => Msg::new("dev-cooling-same").num("percent", change.percent as i32),
    };
    println!("{}", catalog.render(&message));
    Ok(())
}

fn watch(catalog: &Catalog, minutes: u64) -> Result<(), BoxError> {
    let mut watch = Watch::default();
    let mut cooling = Cooling::default();
    let mut recent: std::collections::VecDeque<(u64, i32)> = std::collections::VecDeque::new();
    let until = lenthm_linux::now() + minutes * 60_000_000;
    while lenthm_linux::now() < until {
        let reading = read()?;
        let processor = reading.processor().cloned();
        if let Some(processor) = &processor {
            recent.push_back((reading.time, processor.millis));
            while recent.front().is_some_and(|(time, _)| reading.time.saturating_sub(*time) > SLOPE_WINDOW) {
                recent.pop_front();
            }
            let slope = recent
                .front()
                .filter(|(time, _)| *time < reading.time)
                .map_or(0.0, |(time, millis)| (processor.millis - millis) as f32 / 1000.0 / ((reading.time - time) as f32 / MICROS_PER_MINUTE));
            let hottest = reading.hottest().map(|sensor| sensor.label.clone()).unwrap_or_default();
            let watts = reading.power.map_or_else(|| "—".to_string(), |power| format!("{:.1}", power as f32 / 1000.0));
            println!(
                "{}",
                catalog.render(
                    &Msg::new("dev-watch-line")
                        .num("celsius", processor.millis / 1000)
                        .arg("slope", format!("{slope:+.1}"))
                        .arg("watts", watts)
                        .arg("hottest", hottest)
                        .num("top", reading.hottest().map_or(0, |sensor| sensor.millis / 1000))
                )
            );
            if let (Some(reference), Some(power)) = (reading.reference(), reading.power)
                && let Some(sample) = cooling.observe(reading.time, lenthm_linux::today(), processor.celsius(), reference.celsius(), power)
            {
                println!("{}", catalog.render(&Msg::new("dev-watch-sample").arg("rise", format!("{:.1}", sample.rise)).num("bucket", sample.bucket)));
            }
        }
        for said in watch.push(&reading) {
            println!("{}", catalog.render(&notice(&said)));
        }
        std::thread::sleep(EVERY);
    }
    Ok(())
}

fn run(catalog: &Catalog) -> Result<(), BoxError> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        None | Some("status") => status(catalog),
        Some("watch") => {
            let minutes = match args.get(1) {
                Some(value) => value
                    .parse::<u64>()
                    .ok()
                    .filter(|minutes| (1..=LONGEST).contains(minutes))
                    .ok_or_else(|| Fail::new("dev-bad-minutes").arg("value", value))?,
                None => WATCH_MINUTES,
            };
            watch(catalog, minutes)
        }
        Some("help" | "--help" | "-h") => {
            println!("{}", catalog.text("dev-usage"));
            Ok(())
        }
        Some(other) => Err(Fail::new("dev-bad-option").arg("value", other).into()),
    }
}

fn main() -> ExitCode {
    unsafe { libc::signal(libc::SIGPIPE, libc::SIG_DFL) };
    let catalog = lenthm_linux::catalog();
    match run(&catalog) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{}", catalog.render(&as_msg(e.as_ref(), "dev-error")));
            ExitCode::FAILURE
        }
    }
}
