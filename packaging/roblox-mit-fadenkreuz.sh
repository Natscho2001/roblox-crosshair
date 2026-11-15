#!/bin/sh
# Startet Roblox (Sober) zusammen mit dem Fadenkreuz-Overlay und schließt das
# Overlay wieder, sobald Roblox beendet wird.
# Erwartet roblox-crosshair im selben Ordner wie dieses Skript (z. B. ~/Applications).

DIR="$(dirname "$(readlink -f "$0")")"
BIN="$DIR/roblox-crosshair"
[ -x "$BIN" ] || BIN="$(command -v roblox-crosshair)"

started=0
if [ -n "$BIN" ] && ! "$BIN" --status >/dev/null 2>&1; then
    "$BIN" >/dev/null 2>&1 &
    started=1
fi

flatpak run org.vinegarhq.Sober "$@"
rc=$?

# Nur das Overlay schließen, das dieses Skript selbst gestartet hat.
if [ "$started" = 1 ]; then
    "$BIN" --quit >/dev/null 2>&1
fi
exit $rc
