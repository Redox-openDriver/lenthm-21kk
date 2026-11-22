use drv_i18n::Fail;
use lenthm_core::ecmb;

pub type BoxError = Box<dyn std::error::Error>;

pub const LOCALES: [(&str, &str); 2] = [("ru", include_str!("../../locales/ru.ftl")), ("en", include_str!("../../locales/en.ftl"))];

pub fn catalog() -> drv_i18n::Catalog {
    drv_i18n::Catalog::new(&LOCALES)
}

pub struct Window {
    base: *const u8,
}

impl Window {
    pub fn map() -> Result<Window, BoxError> {
        let protection = common::Prot { read: true, write: false };
        let pointer = unsafe { common::physmap(ecmb::BASE, ecmb::SIZE, protection, common::MemoryType::Uncacheable) }
            .map_err(|e| Fail::new("redox-map-failed").arg("address", format!("{:#x}", ecmb::BASE)).arg("detail", e))?;
        Ok(Window { base: pointer.cast() })
    }

    pub fn snapshot(&self) -> [u8; ecmb::SIZE] {
        let mut copy = [0u8; ecmb::SIZE];
        for (at, byte) in copy.iter_mut().enumerate() {
            *byte = unsafe { std::ptr::read_volatile(self.base.add(at)) };
        }
        copy
    }
}
