#!/bin/sh
set -eu

here=$(cd "$(dirname "$0")" && pwd)
catalogs=$here/locales
. "$here/messages.sh"
language=$(language)
say() { text "$@"; }
fail() { text "$@" >&2; exit 1; }

[ "$(id -u)" -ne 0 ] || fail setup-not-as-root
command -v cargo >/dev/null 2>&1 || fail setup-no-cargo
bin=${XDG_BIN_HOME:-$HOME/.local/bin}
config=${XDG_CONFIG_HOME:-$HOME/.config}
cd "$here/../.."
cargo build --release --locked -p lenthm-linux
install -Dm755 target/release/lenthmd "$bin/lenthmd"
install -Dm755 target/release/lenthm-dev "$bin/lenthm-dev"
if systemctl --user show-environment >/dev/null 2>&1; then
  install -Dm644 "$here/lenthmd.service" "$config/systemd/user/lenthmd.service"
  systemctl --user daemon-reload
  systemctl --user enable --now lenthmd.service >/dev/null 2>&1 || fail setup-service-failed
  say setup-done-systemd
else
  mkdir -p "$config/autostart"
  sed "s|@BIN@|$bin|" "$here/lenthmd.desktop" > "$config/autostart/lenthmd.desktop"
  say setup-done-autostart file="$config/autostart/lenthmd.desktop"
fi
say setup-next-status program="$bin/lenthm-dev"
