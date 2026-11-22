use std::process::ExitCode;
use std::time::{Duration, Instant};

use calor::{Notice, Watch};
use drv_i18n::{Catalog, Msg, as_msg};
use lenthm_core::ecmb;
use lenthm_redox::Window;

const EVERY: Duration = Duration::from_secs(10);

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

fn say(catalog: &Catalog, message: &Msg) {
    eprintln!("{}", catalog.render(message));
}

fn main() -> ExitCode {
    let catalog = lenthm_redox::catalog();
    let window = match Window::map() {
        Ok(window) => window,
        Err(e) => {
            say(&catalog, &as_msg(e.as_ref(), "redox-error"));
            return ExitCode::FAILURE;
        }
    };
    say(&catalog, &Msg::new("redox-started").arg("address", format!("{:#x}", ecmb::BASE)));
    let started = Instant::now();
    let mut watch = Watch::default();
    loop {
        let time = u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX);
        if let Some(reading) = ecmb::reading(&window.snapshot(), time) {
            for said in watch.push(&reading) {
                say(&catalog, &notice(&said));
            }
        }
        std::thread::sleep(EVERY);
    }
}
