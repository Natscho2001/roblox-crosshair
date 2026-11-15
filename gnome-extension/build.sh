#!/bin/sh
# Build the installable extension zip: gnome-extension/build.sh [out-dir]
set -eu
cd "$(dirname "$0")"
UUID=$(sed -n 's/.*"uuid": *"\([^"]*\)".*/\1/p' metadata.json)
OUT=$(realpath "${1:-../dist}")
STAGE=$(mktemp -d)
trap 'rm -rf "$STAGE"' EXIT
mkdir -p "$STAGE/schemas" "$OUT"
cp metadata.json extension.js prefs.js geometry.js "$STAGE/"
cp schemas/*.gschema.xml "$STAGE/schemas/"
glib-compile-schemas --strict "$STAGE/schemas"
rm -f "$OUT/$UUID.shell-extension.zip"
python3 - "$STAGE" "$OUT/$UUID.shell-extension.zip" <<'PY'
import os, sys, zipfile
stage, out = sys.argv[1], sys.argv[2]
with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED) as z:
    for root, _dirs, files in os.walk(stage):
        for f in sorted(files):
            full = os.path.join(root, f)
            z.write(full, os.path.relpath(full, stage))
PY
echo "$OUT/$UUID.shell-extension.zip"
