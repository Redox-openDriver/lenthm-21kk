setup-not-as-root = запусти от своего имени, не от root: служба смотрит датчики с твоими правами
setup-no-cargo = для сборки нужен cargo (rustup.rs)
setup-service-failed = пользовательская служба не запустилась: journalctl --user -u lenthmd
setup-done-systemd = lenthmd работает как твоя пользовательская служба и запускается при каждом входе
setup-done-autostart = lenthmd запускается вместе с сеансом рабочего стола: { $file }
setup-next-status = посмотреть сейчас: { $program } status
remove-purged = удалено вместе с дневником охлаждения
remove-kept = удалено; дневник охлаждения остался в { $dir }
