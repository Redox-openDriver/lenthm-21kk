use std::path::Path;
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use calor::{Cooling, Notice, Watch};
use drv_i18n::{Catalog, Msg};

const EVERY: Duration = Duration::from_secs(10);

static STOP: AtomicBool = AtomicBool::new(false);

extern "C" fn stop(_: libc::c_int) {
    STOP.store(true, Ordering::Relaxed);
}

fn notice(said: &Notice) -> Msg {
    match said {
        Notice::Hot { label, celsius } => Msg::new("thermal-hot").arg("sensor", label).num("celsius", *celsius),
        Notice::Cooled { label, celsius } => Msg::new("thermal-cooled").arg("sensor", label).num("celsius", *celsius),
        Notice::NearCritical { label, celsius, critical } => {
            Msg::new("thermal-near-critical").arg("sensor", label).num("celsius", *celsius).num("critical", *critical)
        }
        Notice::CoolingWorse { percent } => Msg::new("thermal-cooling-worse").num("percent", *percent),
    }
}

fn nap(lasting: Duration) {
    let until = Instant::now() + lasting;
    while !STOP.load(Ordering::Relaxed) && Instant::now() < until {
        std::thread::sleep(Duration::from_millis(200));
    }
}

fn main() -> ExitCode {
    unsafe {
        let handler: extern "C" fn(libc::c_int) = stop;
        libc::signal(libc::SIGINT, handler as libc::sighandler_t);
        libc::signal(libc::SIGTERM, handler as libc::sighandler_t);
    }
    let catalog = lenthm_linux::catalog();
    let say = |catalog: &Catalog, message: &Msg| eprintln!("{}", catalog.render(message));
    let diary = lenthm_linux::diary();
    if let Some(dir) = diary.as_deref().and_then(Path::parent)
        && let Err(e) = std::fs::create_dir_all(dir)
    {
        say(&catalog, &Msg::new("thermal-diary-failed").arg("file", dir.display()).arg("detail", e));
    }
    say(&catalog, &Msg::new("thermal-started").arg("file", diary.as_deref().map_or_else(|| "—".into(), |path| path.display().to_string())));
    let (mut watch, mut cooling, mut complained) = (Watch::default(), Cooling::default(), false);
    while !STOP.load(Ordering::Relaxed) {
        let reading = lenthm_linux::reading(Path::new(lenthm_linux::HWMON), Path::new(lenthm_linux::THERMAL), lenthm_linux::now());
        for said in watch.push(&reading) {
            say(&catalog, &notice(&said));
        }
        let today = lenthm_linux::today();
        if let (Some(processor), Some(reference), Some(power), Some(path)) = (reading.processor(), reading.reference(), reading.power, diary.as_deref())
            && let Some(sample) = cooling.observe(reading.time, today, processor.celsius(), reference.celsius(), power)
        {
            match lenthm_core::diary::keep(path, sample, today) {
                Ok(()) => complained = false,
                Err(e) if !complained => {
                    complained = true;
                    say(&catalog, &Msg::new("thermal-diary-failed").arg("file", path.display()).arg("detail", e));
                }
                Err(_) => {}
            }
            let samples = lenthm_core::diary::samples(path, today).unwrap_or_default();
            if let Some(said) = watch.cooling(calor::assess(&samples, today)) {
                say(&catalog, &notice(&said));
            }
        }
        nap(EVERY);
    }
    say(&catalog, &Msg::new("thermal-stopped"));
    ExitCode::SUCCESS
}
