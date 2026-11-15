#!/bin/bash
# Load test: install the extension zip into a throwaway HOME, start a headless
# GNOME Shell (Wayland, virtual monitor), and check that the extension becomes
# active, survives settings changes and disable/enable, and logs no JS errors.
# Usage: tests/headless_gnome.sh path/to/<uuid>.shell-extension.zip
set -euo pipefail
ZIP=$(realpath "$1")
UUID=roblox-crosshair@natscho2001.github.io
HOME=$(mktemp -d)
XDG_RUNTIME_DIR=$(mktemp -d)
chmod 700 "$XDG_RUNTIME_DIR"
export HOME XDG_RUNTIME_DIR ZIP UUID
LOG="$HOME/shell.log"
export LOG

dbus-run-session -- bash -euo pipefail <<'INNER'
gnome-extensions install --force "$ZIP"
gsettings set org.gnome.shell disable-user-extensions false
gsettings set org.gnome.shell enabled-extensions "['$UUID']"

gnome-shell --headless --wayland --virtual-monitor 1920x1080 >"$LOG" 2>&1 &
SHELL_PID=$!
trap 'kill $SHELL_PID 2>/dev/null || true' EXIT

# Ask the shell directly (the `gnome-extensions` CLI needs an extra service in 46).
shell_call() {
    gdbus call --session --dest org.gnome.Shell --object-path /org/gnome/Shell \
        --method "org.gnome.Shell.Extensions.$1" "$UUID" 2>/dev/null
}
# ExtensionState: 1 = ACTIVE/ENABLED, 3 = ERROR, 4 = OUT_OF_DATE …
state() { shell_call GetExtensionInfo | grep -oE "'state': <[0-9.]+>" | grep -oE "[0-9]+" | head -1; }
info_error() { shell_call GetExtensionInfo | grep -oE "'error': <'[^']*'>" || true; }
for _ in $(seq 90); do
    [ "$(state)" = "1" ] && break
    sleep 1
done
echo "state after start: $(state) $(info_error)"
if [ "$(state)" != "1" ]; then echo "FAIL: extension not active"; shell_call GetExtensionInfo; cat "$LOG"; exit 1; fi

S="gsettings --schemadir $HOME/.local/share/gnome-shell/extensions/$UUID/schemas"
K=org.gnome.shell.extensions.roblox-crosshair
for kv in "show-when always" "follow pointer" "follow center" "follow auto" "shape dot" "shape cross" \
          "outline false" "center-on window" "visible false" "visible true" "quick-toggle false" \
          "quick-toggle true" "show-when roblox-running"; do
    # shellcheck disable=SC2086
    $S set $K $kv
    sleep 0.3
done
shell_call DisableExtension >/dev/null; sleep 1
shell_call EnableExtension >/dev/null; sleep 2
echo "state after settings + disable/enable: $(state) $(info_error)"
if [ "$(state)" != "1" ]; then echo "FAIL: not active after re-enable"; cat "$LOG"; exit 1; fi

if grep -nE "JS ERROR|JS WARNING.*roblox-crosshair|roblox-crosshair.*(Error|TypeError)" "$LOG"; then
    echo "FAIL: errors in the shell log"; exit 1
fi
echo "OK: extension loaded and exercised without JS errors"
INNER
