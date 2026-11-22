#!/bin/sh
set -eu

here=$(cd "$(dirname "$0")" && pwd)
catalogs=$here/locales
. "$here/messages.sh"
language=$(language)
say() { text "$@"; }

bin=${XDG_BIN_HOME:-$HOME/.local/bin}
config=${XDG_CONFIG_HOME:-$HOME/.config}
state=${XDG_STATE_HOME:-$HOME/.local/state}
if [ -e "$config/systemd/user/lenthmd.service" ]; then
  systemctl --user disable --now lenthmd.service >/dev/null 2>&1 || true
  rm -f "$config/systemd/user/lenthmd.service"
  systemctl --user daemon-reload >/dev/null 2>&1 || true
fi
rm -f "$config/autostart/lenthmd.desktop" "$bin/lenthmd" "$bin/lenthm-dev"
if [ "${1:-}" = --purge ]; then
  rm -rf "$state/lenthm"
  say remove-purged
else
  say remove-kept dir="$state/lenthm"
fi
