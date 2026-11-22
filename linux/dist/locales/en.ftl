setup-not-as-root = run it as yourself, not as root: the service watches the sensors with your own rights
setup-no-cargo = cargo is needed to build it (rustup.rs)
setup-service-failed = the user service did not start: journalctl --user -u lenthmd
setup-done-systemd = lenthmd runs as your user service and starts with every login
setup-done-autostart = lenthmd starts with your desktop session: { $file }
setup-next-status = see it now: { $program } status
remove-purged = removed, together with the cooling diary
remove-kept = removed; the cooling diary stays in { $dir }
