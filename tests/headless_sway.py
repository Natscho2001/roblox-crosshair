#!/usr/bin/env python3
"""Integration test: run the overlay under a headless Sway and check the pixels.

Usage: tests/headless_sway.py path/to/roblox-crosshair
Needs `sway` and `grim`. Starts its own Sway (WLR_BACKENDS=headless), so it works
on CI runners and servers without a display. Exits non-zero on any failure.
"""

import os
import shutil
import subprocess
import sys
import tempfile
import time

BIN = os.path.abspath(sys.argv[1])
run = tempfile.mkdtemp(prefix="rcw", dir="/tmp")  # short path: socket names must fit sun_path
os.chmod(run, 0o700)
home = tempfile.mkdtemp(prefix="rch")
cfg = os.path.join(run, "sway.cfg")
open(cfg, "w").write("output HEADLESS-1 resolution 1920x1080 scale 1\ndefault_border none\n")
base_env = dict(os.environ, XDG_RUNTIME_DIR=run, WLR_BACKENDS="headless", WLR_LIBINPUT_NO_DEVICES="1",
                WLR_RENDERER="pixman", HOME=home, XDG_CONFIG_HOME=os.path.join(home, "cfg"))
sway = subprocess.Popen(["sway", "-c", cfg], env=base_env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
failures = []


def check(cond, msg):
    print(("ok   " if cond else "FAIL ") + msg)
    if not cond:
        failures.append(msg)


def wait_for(cond, t=10):
    end = time.time() + t
    while time.time() < end:
        if cond():
            return True
        time.sleep(0.1)
    return False


try:
    assert wait_for(lambda: any(f.startswith("wayland-") and not f.endswith(".lock") for f in os.listdir(run))
                    and any(f.startswith("sway-ipc") for f in os.listdir(run))), "sway did not start"
    display = next(f for f in os.listdir(run) if f.startswith("wayland-") and not f.endswith(".lock"))
    env = dict(base_env, WAYLAND_DISPLAY=display,
               SWAYSOCK=os.path.join(run, next(f for f in os.listdir(run) if f.startswith("sway-ipc"))))

    def sh(*a, check_rc=True):
        return subprocess.run(list(a), env=env, capture_output=True, text=True, check=check_rc)

    def shot():
        p = os.path.join(run, "shot.ppm")
        sh("grim", "-t", "ppm", p)
        head, dims, _maxval, px = open(p, "rb").read().split(b"\n", 3)
        w, h = map(int, dims.split())
        return w, h, px

    def green_centroid(w, h, px, box=120):
        sx = sy = n = 0
        for y in range(max(0, h // 2 - box), min(h, h // 2 + box)):
            for x in range(max(0, w // 2 - box), min(w, w // 2 + box)):
                i = (y * w + x) * 3
                if px[i + 1] > 200 and px[i] < 120 and px[i + 2] < 120:
                    sx += x + 0.5
                    sy += y + 0.5
                    n += 1
        return (sx / n - w / 2, sy / n - h / 2, n) if n else (None, None, 0)

    overlay = subprocess.Popen([BIN], env=env)
    check(wait_for(lambda: sh(BIN, "--status", check_rc=False).returncode == 0), "overlay starts and answers --status")
    time.sleep(0.5)
    sizes = []
    for scale in ["1", "1.25", "1.5", "2"]:
        sh("swaymsg", "output", "HEADLESS-1", "scale", scale)
        time.sleep(0.8)
        dx, dy, n = green_centroid(*shot())
        sizes.append(n)
        check(n > 0 and abs(dx) <= 0.5 and abs(dy) <= 0.5,
              f"scale {scale}: crosshair centred (dx={dx}, dy={dy}, {n} px)")
    check(sizes == sorted(sizes) and sizes[0] < sizes[-1], f"crosshair grows with the scale {sizes}")
    sh("swaymsg", "output", "HEADLESS-1", "scale", "1")
    time.sleep(0.5)

    check(sh(BIN, "--toggle").stdout.strip() == "hidden", "--toggle hides")
    time.sleep(0.5)
    check(green_centroid(*shot())[2] == 0, "hidden overlay draws nothing")
    check(sh(BIN, "--toggle").stdout.strip() == "visible", "--toggle shows again")
    time.sleep(0.5)
    check(green_centroid(*shot())[2] > 0, "visible again")

    os.makedirs(os.path.join(home, "cfg", "roblox-crosshair"), exist_ok=True)
    with open(os.path.join(home, "cfg", "roblox-crosshair", "config.toml"), "w") as f:
        f.write('shape = "dot"\nsize = 12\noffset_x = 40\noffset_y = -20\n')
    check(sh(BIN, "--reload").stdout.strip() == "visible", "--reload accepted")
    time.sleep(0.6)
    dx, dy, n = green_centroid(*shot())
    check(n > 0 and abs(dx - 40) <= 0.5 and abs(dy + 20) <= 0.5, f"offset applied after reload (dx={dx}, dy={dy})")

    check("läuft bereits" in sh(BIN).stdout, "second start refuses to run twice")
    check(sh(BIN, "--quit").stdout.strip() == "quit", "--quit accepted")
    check(overlay.wait(timeout=5) == 0, "overlay exits cleanly")
    check(sh(BIN, "--status", check_rc=False).returncode == 1, "--status reports not running")
finally:
    sway.terminate()
    sway.wait(timeout=5)
    shutil.rmtree(run, ignore_errors=True)
    shutil.rmtree(home, ignore_errors=True)

sys.exit(1 if failures else 0)
