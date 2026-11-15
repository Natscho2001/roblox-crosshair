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

state() { gnome-extensions info "$UUID" 2>/dev/null | sed -n 's/^ *State: *//p'; }
for _ in $(seq 90); do
    case "$(state)" in ACTIVE|ENABLED) break ;; esac
    sleep 1
done
echo "state after start: $(state)"
case "$(state)" in ACTIVE|ENABLED) ;; *) echo "FAIL: extension not active"; cat "$LOG"; exit 1 ;; esac

S="gsettings --schemadir $HOME/.local/share/gnome-shell/extensions/$UUID/schemas"
K=org.gnome.shell.extensions.roblox-crosshair
for kv in "show-when always" "shape dot" "shape cross" "outline false" "center-on window" \
          "visible false" "visible true" "quick-toggle false" "quick-toggle true" "show-when roblox-running"; do
    # shellcheck disable=SC2086
    $S set $K $kv
    sleep 0.3
done
gnome-extensions disable "$UUID"; sleep 1
gnome-extensions enable "$UUID"; sleep 2
echo "state after settings + disable/enable: $(state)"
case "$(state)" in ACTIVE|ENABLED) ;; *) echo "FAIL: not active after re-enable"; cat "$LOG"; exit 1 ;; esac

if grep -nE "JS ERROR|JS WARNING.*roblox-crosshair|roblox-crosshair.*(Error|TypeError)" "$LOG"; then
    echo "FAIL: errors in the shell log"; exit 1
fi
echo "OK: extension loaded and exercised without JS errors"
INNER
